use crate::HirLowerer;
use hir_model::{self as hir};
use parser_models::{ast, scope::NodeId};
use soul_utils::{error::{SoulError, SoulErrorKind}, span::Span, vec_map::VecMap};

impl HirLowerer {
    pub(crate) fn lower_expression(
        &mut self,
        expression: &ast::Expression,
    ) -> Option<hir::ExpressionId> {
        let span = expression.get_span();
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
            ast::ExpressionKind::Array(array) => {
                let id = self.expect_node_id(array.id, span)?;
                let mut nodes = Vec::with_capacity(array.values.len());
                for value in &array.values {
                    nodes.push((self.lower_expression(value)?, value.get_span()));
                }
                (id, hir::ExpressionKind::Array(hir::Array{
                    id,
                    values: nodes,
                }))
            }
            ast::ExpressionKind::Variable {
                id,
                ident: _,
                resolved,
            } => {
                let id = self.expect_node_id(*id, span)?;
                let resolved = self.expect_node_id(*resolved, expression.get_span())?;
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
                self.push_block(body, block.span);
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
                    ast::ReturnKind::Continue => {
                        if value.is_some() {
                            self.log_error(SoulError::new(
                                "continue should not have an expression", 
                                SoulErrorKind::InvalidContext, 
                                Some(expression.get_span()),
                            ));
                        }
                        hir::ExpressionKind::Continue(id)
                    }
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
            ast::ExpressionKind::ExternalExpression(_) => todo!("impl externalExpression"),
        };

        let expression = hir::Expression::with_meta_data(kind, expression.get_meta_data().clone());

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

    fn lower_tuple(&mut self, tuple: &Vec<ast::Expression>) -> Option<Vec<NodeId>> {
        let mut nodes = Vec::with_capacity(tuple.len());
        for value in tuple {
            nodes.push(self.lower_expression(value)?);
        }
        Some(nodes)
    }

    fn lower_function_call(
        &mut self,
        function_call: &ast::FunctionCall,
        span: Span,
    ) -> Option<(NodeId, hir::ExpressionKind)> {
        let id = self.expect_node_id(function_call.id, span)?;
        let resolved = self.expect_node_id(function_call.resolved, span)?;

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
                resolved: resolved,
            }),
        ))
    }
}
