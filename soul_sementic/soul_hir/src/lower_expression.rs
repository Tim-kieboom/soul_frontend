use crate::HirLowerer;
use hir_model::{self as hir};
use parser_models::{ast, scope::NodeId};
use soul_utils::{Ident, span::Span, vec_map::VecMap};

impl HirLowerer {
    pub(crate) fn lower_expression(
        &mut self,
        expression: &ast::Expression,
    ) -> Option<hir::ExpressionId> {
        let span = expression.span;
        let (id, kind) = match &expression.node {
            ast::ExpressionKind::Default(node_id) => {
                let id = self.expect_node_id(*node_id, span)?;
                (id, hir::ExpressionKind::Default)
            }
            ast::ExpressionKind::Literal((node_id, literal)) => {
                let id = self.expect_node_id(*node_id, span)?;
                (id, hir::ExpressionKind::Literal(literal.clone()))
            }
            ast::ExpressionKind::Index(ast::Index {
                id,
                collection,
                index,
            }) => {
                let id = self.expect_node_id(*id, span)?;
                (
                    id,
                    hir::ExpressionKind::Index(hir::Index {
                        collection: self.lower_expression(collection)?,
                        index: self.lower_expression(index)?,
                    }),
                )
            }
            ast::ExpressionKind::FunctionCall(function_call) => {
                self.lower_function_call(function_call, span)?
            }
            ast::ExpressionKind::FieldAccess(field_access) => {
                self.lower_field_access(field_access, span)?
            }
            ast::ExpressionKind::Variable {
                id,
                ident: _,
                resolved,
            } => {
                let id = self.expect_node_id(*id, span)?;
                let resolved = self.expect_node_id(*resolved, expression.span)?;
                (id, hir::ExpressionKind::ResolvedVariable(resolved))
            }
            ast::ExpressionKind::If(r#if) => self.lower_if(r#if, span)?,
            ast::ExpressionKind::While(r#while) => self.lower_while(r#while, span)?,
            ast::ExpressionKind::Block(block) => {
                let id = self.expect_node_id(block.node_id, span)?;
                let prev_scope = self.push_scope();
                let body = self.lower_block(block, span)?;
                self.pop_scope(prev_scope);

                let body_id = body.id;
                self.push_block(body);
                (id, hir::ExpressionKind::Block(body_id))
            }
            ast::ExpressionKind::Unary(unary) => {
                let id = self.expect_node_id(unary.id, span)?;
                (
                    id,
                    hir::ExpressionKind::Unary(hir::Unary {
                        operator: unary.operator.clone(),
                        expression: self.lower_expression(&unary.expression)?,
                    }),
                )
            }
            ast::ExpressionKind::Deref { id, inner } => {
                let id = self.expect_node_id(*id, span)?;
                (
                    id,
                    hir::ExpressionKind::DeRef(self.lower_expression(inner)?),
                )
            }
            ast::ExpressionKind::Binary(binary) => {
                let id = self.expect_node_id(binary.id, span)?;
                (
                    id,
                    hir::ExpressionKind::Binary(hir::Binary {
                        operator: binary.operator.clone(),
                        left: self.lower_expression(&binary.left)?,
                        right: self.lower_expression(&binary.right)?,
                    }),
                )
            }
            ast::ExpressionKind::ReturnLike(return_like) => {
                let id = self.expect_node_id(return_like.id, span)?;
                let value = match &return_like.value {
                    Some(val) => Some(self.lower_expression(val)?),
                    None => None,
                };
                let hir_return_like = hir::ReturnLike {
                    value,
                    id: id,
                    kind: return_like.kind,
                };

                let kind = match return_like.kind {
                    ast::ReturnKind::Break => hir::ExpressionKind::Break(hir_return_like),
                    ast::ReturnKind::Return => hir::ExpressionKind::Return(hir_return_like),
                    ast::ReturnKind::Continue => hir::ExpressionKind::Continue(hir_return_like),
                };
                (id, kind)
            }
            ast::ExpressionKind::Ref {
                id,
                is_mutable,
                expression,
            } => {
                let id = self.expect_node_id(*id, span)?;
                (
                    id,
                    hir::ExpressionKind::Ref(hir::Ref {
                        mutable: *is_mutable,
                        expression: self.lower_expression(expression)?,
                    }),
                )
            }
            ast::ExpressionKind::ExpressionGroup { id, group } => {
                let id = self.expect_node_id(*id, span)?;
                (id, self.lower_group_expression(group)?)
            }
            ast::ExpressionKind::TypeNamespace(_) => todo!("impl TytpeNamespace"),
            ast::ExpressionKind::StructConstructor(_) => todo!("impl struct constructor"),
            ast::ExpressionKind::ExternalExpression(_) => todo!("impl externalExpression"),
        };

        let expression = hir::Expression::with_atribute(kind, span, expression.attributes.clone());

        self.push_expression(id, expression);
        Some(id)
    }

    pub(crate) fn lower_block(&mut self, block: &ast::Block, span: Span) -> Option<hir::Block> {
        let id = self.expect_node_id(block.node_id, span)?;

        let mut statements = VecMap::new();

        for ast_statement in &block.statements {
            if let Some((node_id, statement)) = self.lower_statement(ast_statement) {
                statements.insert(node_id, statement);
            }
        }

        Some(hir::Block {
            id,
            statements,
            modifier: block.modifier,
            scope_id: self.current_scope,
        })
    }

    fn lower_group_expression(
        &mut self,
        expression_group: &ast::ExpressionGroup,
    ) -> Option<hir::ExpressionKind> {
        let group = match expression_group {
            ast::ExpressionGroup::Array(array) => {
                let mut nodes = Vec::with_capacity(array.values.len());
                for value in &array.values {
                    nodes.push(self.lower_expression(value)?);
                }
                hir::ExpressionGroup::Array(nodes)
            }
            ast::ExpressionGroup::Tuple(tuple) => {
                let mut values = Vec::with_capacity(tuple.len());
                for (i, value) in tuple.iter().enumerate() {
                    let ident = Ident::new(i.to_string(), value.span);
                    values.push((ident, self.lower_expression(value)?));
                }
                hir::ExpressionGroup::Tuple {
                    values,
                    insert_defaults: false,
                }
            }
            ast::ExpressionGroup::NamedTuple(named_tuple) => {
                let mut values = Vec::with_capacity(named_tuple.values.len());
                for (ident, value) in &named_tuple.values {
                    values.push((ident.clone(), self.lower_expression(value)?));
                }
                hir::ExpressionGroup::Tuple {
                    values,
                    insert_defaults: named_tuple.insert_defaults,
                }
            }
        };

        Some(hir::ExpressionKind::ExpressionGroup(group))
    }

    fn lower_tuple(&mut self, tuple: &ast::Tuple) -> Option<Vec<NodeId>> {
        let mut nodes = Vec::with_capacity(tuple.len());
        for value in tuple {
            nodes.push(self.lower_expression(value)?);
        }
        Some(nodes)
    }

    fn lower_field_access(
        &mut self,
        access_field: &ast::FieldAccess,
        span: Span,
    ) -> Option<(NodeId, hir::ExpressionKind)> {
        let id = self.expect_node_id(access_field.id, span)?;
        Some((
            id,
            hir::ExpressionKind::FieldAccess(hir::FieldAccess {
                field: access_field.field.clone(),
                parent: self.lower_expression(&access_field.parent)?,
            }),
        ))
    }

    fn lower_function_call(
        &mut self,
        function_call: &ast::FunctionCall,
        span: Span,
    ) -> Option<(NodeId, hir::ExpressionKind)> {
        let id = self.expect_node_id(function_call.id, span)?;

        let callee = match &function_call.callee {
            Some(val) => Some(self.lower_expression(val)?),
            None => None,
        };

        Some((
            id,
            hir::ExpressionKind::FunctionCall(hir::FunctionCall {
                name: function_call.name.clone(),
                callee,
                arguments: self.lower_tuple(&function_call.arguments)?,
                generics: self.lower_generic_define(&function_call.generics)?,
            }),
        ))
    }
}
