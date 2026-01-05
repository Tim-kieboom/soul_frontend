use crate::HirLowerer;
use soul_hir::{self as hir, ExpressionId, HirBlockId, HirId};
use soul_utils::{SoulError, SoulErrorKind, SoulResult, Span, soul_names::{OperatorMethodes, TypeModifier}};
use soul_ast::abstract_syntax_tree::{self as ast, Ident};

impl<'hir> HirLowerer<'hir> {

    pub(super) fn lower_expression(
        &mut self,
        expression: &ast::Expression,
    ) -> Option<hir::ExpressionId> {
        let kind = self.lower_expression_kind(expression)?;
        Some(self.add_expression(hir::Expression::with_atribute(kind, expression.span, expression.attributes.clone())))
    }

    
    pub(super) fn lower_op_expression(&mut self, value: Option<&ast::Expression>) -> Option<Option<ExpressionId>> {
        match value {
            Some(val) => Some(Some(
                self.lower_expression(val)?
            )),
            None => Some(None),
        }
    }

    pub(super) fn lower_expression_tuple(&mut self, expressions: &[ast::Expression]) -> Option<Vec<ExpressionId>> {
        let mut hir_expressions = Vec::with_capacity(expressions.len());
        for el in expressions {
            hir_expressions.push(self.lower_expression(el)?);
        } 

        Some(hir_expressions)
    }

    pub(super) fn lower_expression_as_fall_block(&mut self, expression: &ast::Expression) -> HirBlockId {
        let fall = ast::ReturnLike{ value: Some(Box::new(expression.clone())), kind: ast::ReturnKind::Fall };
        let statement = ast::Statement::from_expression(ast::Expression::new(ast::ExpressionKind::ReturnLike(fall), expression.span));

        let body = ast::Block{ 
            modifier: TypeModifier::Mut, 
            statements: vec![statement], 
            scope_id: None,
        };
        self.lower_block(&body, vec![])
    }

    pub(super) fn lower_expression_named_tuple(&mut self, expressions: &[(Ident, ast::Expression)]) -> Option<Vec<(Ident, ExpressionId)>> {
        let mut hir_expressions = Vec::with_capacity(expressions.len());
        for (name, el) in expressions {
            hir_expressions.push(
                (name.clone(), self.lower_expression(el)?)
            );
        } 

        Some(hir_expressions)
    }

    fn lower_expression_kind(&mut self, expression: &ast::Expression) -> Option<hir::ExpressionKind> {
        let span = expression.span;
        
        Some(match &expression.node {
            ast::ExpressionKind::Literal(literal) => hir::ExpressionKind::Literal(literal.clone()),
            ast::ExpressionKind::Variable { ident, .. } => self.lower_variable_expression(ident)?,
            ast::ExpressionKind::Index(index) => self.lower_index(index, span)?,
            ast::ExpressionKind::FunctionCall(function_call) => self.lower_function_call(function_call)?,
            ast::ExpressionKind::StructConstructor(ctor) => self.lower_struct_constructor(ctor)?,
            ast::ExpressionKind::FieldAccess(access_field) => {
                hir::ExpressionKind::FieldAccess(hir::FieldAccess {
                    field: access_field.field.clone(),
                    reciever: self.lower_expression(&access_field.object)?,
                })
            }
            ast::ExpressionKind::StaticFieldAccess(static_field_access) => {
                hir::ExpressionKind::StaticFieldAccess(hir::StaticFieldAccess {
                    field: static_field_access.field.clone(),
                    reciever: self.get_hir_type(&static_field_access.object)?,
                })
            }
            ast::ExpressionKind::If(r#if) => hir::ExpressionKind::If(self.get_hir_if(r#if)?),
            ast::ExpressionKind::Deref(_) => todo!(),
            ast::ExpressionKind::Block(_) => todo!(), 
            ast::ExpressionKind::While(_) => todo!(),
            ast::ExpressionKind::Unary(_) => todo!(), 
            ast::ExpressionKind::Ref { .. } => todo!(),
            ast::ExpressionKind::Ternary(_) => todo!(), 
            ast::ExpressionKind::ReturnLike(_) => todo!(), 
            ast::ExpressionKind::ExpressionGroup(_) => todo!(), 
            
            ast::ExpressionKind::For(_) 
            | ast::ExpressionKind::Default 
            | ast::ExpressionKind::Match(_)
            | ast::ExpressionKind::Binary(_) 
            | ast::ExpressionKind::Lambda(_)
            | ast::ExpressionKind::ExternalExpression(_) => todo!("impl lower's"),


            ast::ExpressionKind::Empty => {
                self.log_error(SoulError::new(
                    "Empty expression is not allowed here",
                    SoulErrorKind::InvalidExpression,
                    Some(expression.span),
                ));
                return None;
            }
        })
    }

    fn lower_variable_expression(&mut self, ident: &Ident) -> Option<hir::ExpressionKind> {
        
        let id = match self.try_get_variable(ident)  {
            Ok(val) => val,
            Err(err) => {
                self.log_error(err);
                return None
            }
        };

        Some(
            hir::ExpressionKind::ResolvedVariable(id)
        )
    }

    fn try_get_variable(&mut self, ident: &Ident) -> SoulResult<HirId> {
        let (locale_id, scope_id) = self.try_get_locale(ident)?;
        let body = self.current_body()?;
        match body {
            soul_hir::Body::Block(block) => todo!(),
            soul_hir::Body::Expression(hir_id) => ,
        }
    }

    fn lower_index(&mut self, index: &ast::Index, span: Span) -> Option<hir::ExpressionKind> {
        const NAME: &str = OperatorMethodes::Index.as_str();
        let callee = self.lower_expression(&index.collection)?;
        let index = self.lower_expression(&index.index)?;

        Some(hir::ExpressionKind::FunctionCall(hir::FunctionCall {
            callee: Some(callee),
            arguments: vec![index],
            name: Ident::new(
                NAME.to_string(), 
                span,
            ),
        }))
    }

    fn lower_function_call(&mut self, function_call: &ast::FunctionCall) -> Option<hir::ExpressionKind> {
        let arguments = self.lower_expression_tuple(&function_call.arguments.values)?;
        let callee = self.lower_op_expression(function_call.callee.as_deref())?;

        Some(hir::ExpressionKind::FunctionCall(hir::FunctionCall {
            name: function_call.name.clone(),
            arguments,
            callee,
        }))
    }

        
    fn lower_struct_constructor(&mut self, ctor: &ast::StructConstructor) -> Option<hir::ExpressionKind> {
        let fields = self.lower_expression_named_tuple(&ctor.arguments.values)?;

        Some(hir::ExpressionKind::StructContructor(hir::StructContructor {
            ty: self.get_hir_type(&ctor.calle)?,
            fields,
            insert_defaults: ctor.arguments.insert_defaults,
        }))
    }

    fn get_hir_if(&mut self, ast_if: &ast::If) -> Option<hir::If> {
        let mut hir_if = hir::If {
            else_arm: None,
            body: self.lower_block(&ast_if.block, vec![]),
            condition: self.lower_expression(&ast_if.condition)?,
        };

        let mut tail = &mut hir_if.else_arm;
        let mut current = ast_if.else_branchs.as_ref();

        while let Some(branch) = current {
            let next = match &branch.node {
                ast::ElseKind::ElseIf(elif) => {
                    let inner = self.get_hir_if(&elif.node)?;
                    let arm = hir::IfArm::ElseIf(inner);
                    current = elif.node.else_branchs.as_ref();
                    arm
                }
                ast::ElseKind::Else(el) => {
                    let arm = hir::IfArm::Else(self.lower_block(&el.node, vec![]));
                    current = None;
                    arm
                }
            };

            *tail = Some(Box::new(next));
            tail = match tail.as_mut().unwrap().as_mut() {
                hir::IfArm::ElseIf(i) => &mut i.else_arm,
                hir::IfArm::Else(_) => break,
            };
        }

        todo!()
    }

}