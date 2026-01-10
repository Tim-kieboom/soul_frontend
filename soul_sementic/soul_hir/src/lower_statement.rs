use crate::HirLowerer;
use hir_model::{self as hir, ItemHelper, StatementHelper, Visibility};
use parser_models::{ast, scope::NodeId};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    soul_names::TypeModifier,
    span::{Attribute, Span},
};

impl HirLowerer {
    pub(crate) fn lower_globals_statements(&mut self, statement: &ast::Statement) -> Option<()> {
        const IS_GLOBAL: bool = true;

        let span = statement.span;
        let (id, item) = match &statement.node {
            ast::StatementKind::Import(import) => {
                let ast::Import { id, paths } = import.clone();
                let id = self.expect_node_id(id, span)?;
                (id, hir::Item::new_import(hir::Import { id, paths }, span))
            }
            ast::StatementKind::Variable(variable) => {
                let (id, var) = self.lower_variable(IS_GLOBAL, variable, span)?;
                (
                    id,
                    hir::Item::new_variable(var, span, statement.attributes.clone()),
                )
            }
            ast::StatementKind::Function(function) => {
                let (id, func) = self.lower_function(IS_GLOBAL, function, span)?;
                (
                    id,
                    hir::Item::new_function(func, span, statement.attributes.clone()),
                )
            }
            ast::StatementKind::Expression { .. } => {
                self.log_error(SoulError::new(
                    "can not have Expression in global scope",
                    SoulErrorKind::InvalidContext,
                    Some(span),
                ));
                return None;
            }
            ast::StatementKind::Assignment(_) => {
                self.log_error(SoulError::new(
                    "can not have Assignment in global scope",
                    SoulErrorKind::InvalidContext,
                    Some(span),
                ));
                return None;
            }
        };

        self.push_item(id, item);
        Some(())
    }

    pub(crate) fn lower_statement(
        &mut self,
        statement: &ast::Statement,
    ) -> Option<(NodeId, hir::Statement)> {
        const IS_GLOBAL: bool = false;

        let span = statement.span;
        match &statement.node {
            ast::StatementKind::Import(import) => {
                let ast::Import { id, paths } = import.clone();
                let id = self.expect_node_id(id, span)?;
                Some((
                    id,
                    hir::Statement::new_import(
                        hir::Import { id, paths },
                        span,
                        statement.attributes.clone(),
                    ),
                ))
            }
            ast::StatementKind::Variable(variable) => Some(
                self.lower_variable(IS_GLOBAL, variable, span)
                    .map(|(id, var)| {
                        (
                            id,
                            hir::Statement::new_variable(var, span, statement.attributes.clone()),
                        )
                    })?,
            ),
            ast::StatementKind::Function(function) => self
                .lower_function(IS_GLOBAL, function, span)
                .map(|(id, func)| {
                    (
                        id,
                        hir::Statement::new_function(func, span, statement.attributes.clone()),
                    )
                }),
            ast::StatementKind::Assignment(assignment) => {
                self.lower_assignment(assignment, span, statement.attributes.clone())
            }
            ast::StatementKind::Expression { id, expression } => {
                let id = self.expect_node_id(*id, span)?;
                let expr_id = self.lower_expression(expression)?;
                let statement_expr = hir::StatementExpression {
                    id,
                    expression: expr_id,
                };
                Some((
                    id,
                    hir::Statement::new_expression(
                        statement_expr,
                        span,
                        statement.attributes.clone(),
                    ),
                ))
            }
        }
    }

    fn lower_assignment(
        &mut self,
        assignment: &ast::Assignment,
        span: Span,
        attribute: Vec<Attribute>,
    ) -> Option<(NodeId, hir::Statement)> {
        let id = self.expect_node_id(assignment.node_id, span)?;

        Some((
            id,
            hir::Statement::new_assign(
                hir::Assign {
                    id,
                    left: self.lower_expression(&assignment.left)?,
                    right: self.lower_expression(&assignment.right)?,
                },
                span,
                attribute,
            ),
        ))
    }

    fn lower_variable(
        &mut self,
        global: bool,
        variable: &ast::Variable,
        span: Span,
    ) -> Option<(NodeId, hir::Variable)> {
        let id = self.expect_node_id(variable.node_id, span)?;

        if global {
            if variable.ty.get_modifier() == TypeModifier::Mut {
                self.log_error(SoulError::new(
                    format!(
                        "global variables can not be '{}'",
                        TypeModifier::Mut.as_str()
                    ),
                    SoulErrorKind::InvalidContext,
                    Some(span),
                ));
            }

            self.current_item = Some(id);
        }

        let ty = match &variable.ty {
            ast::VarTypeKind::NonInveredType(soul_type) => {
                hir::VarTypeKind::NonInveredType(self.lower_type(soul_type)?)
            }
            ast::VarTypeKind::InveredType(type_modifier) => {
                hir::VarTypeKind::InveredType(*type_modifier)
            }
        };

        let value = match &variable.initialize_value {
            Some(val) => Some(self.lower_expression(val)?),
            None => None,
        };

        Some((
            id,
            hir::Variable {
                id,
                ty,
                value,
                name: variable.name.clone(),
                vis: Visibility::from_name(&variable.name),
            },
        ))
    }

    fn lower_function(
        &mut self,
        global: bool,
        function: &ast::Function,
        span: Span,
    ) -> Option<(NodeId, hir::Function)> {
        let id = self.expect_node_id(function.node_id, span)?;

        if global {
            self.current_item = Some(id);
        }

        let signature = self.lower_function_signature(&function.signature.node)?;

        let prev_scope = self.push_scope();
        let block = self.lower_block(&function.block, span)?;
        self.pop_scope(prev_scope);

        let body_id = block.id;
        self.push_block(block);

        if global {
            self.current_item = None;
        }

        Some((
            id,
            hir::Function {
                id,
                signature,
                body: body_id,
            },
        ))
    }

    fn lower_function_signature(
        &mut self,
        function_signature: &ast::FunctionSignature,
    ) -> Option<hir::FunctionSignature> {
        Some(hir::FunctionSignature {
            name: function_signature.name.clone(),
            methode_type: self.lower_type(&function_signature.methode_type)?,
            function_kind: function_signature.function_kind,
            return_type: self.lower_type(&function_signature.return_type)?,
            parameters: self.lower_named_tuple_type(&function_signature.parameters)?,
            generics: self.lower_generic_declare(&function_signature.generics)?,
            vis: Visibility::from_name(&function_signature.name),
        })
    }
}
