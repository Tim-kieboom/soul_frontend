use crate::HirLowerer;
use hir_model::{self as hir, ItemHelper, StatementHelper, Visibility};
use parser_models::{ast, scope::NodeId};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    soul_names::TypeModifier,
    span::{NodeMetaData, Span},
};

impl HirLowerer {
    pub(crate) fn lower_globals_statements(&mut self, statement: &ast::Statement) -> Option<()> {
        const IS_GLOBAL: bool = true;

        let span = statement.get_span();
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
                    hir::Item::new_variable(var, statement.get_meta_data().clone()),
                )
            }
            ast::StatementKind::Function(function) => {
                let (id, func) = self.lower_function(IS_GLOBAL, function, span)?;
                (
                    id,
                    hir::Item::new_function(func, statement.get_meta_data().clone()),
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

        let span = statement.get_span();
        match &statement.node {
            ast::StatementKind::Import(import) => {
                let ast::Import { id, paths } = import.clone();
                let id = self.expect_node_id(id, span)?;
                Some((
                    id,
                    hir::Statement::new_import(
                        hir::Import { id, paths },
                        statement.get_meta_data().clone()
                    ),
                ))
            }
            ast::StatementKind::Variable(variable) => Some(
                self.lower_variable(IS_GLOBAL, variable, span)
                    .map(|(id, var)| {
                        (
                            id,
                            hir::Statement::new_variable(var, statement.get_meta_data().clone()),
                        )
                    })?,
            ),
            ast::StatementKind::Function(function) => self
                .lower_function(IS_GLOBAL, function, span)
                .map(|(id, func)| {
                    (
                        id,
                        hir::Statement::new_function(func, statement.get_meta_data().clone()),
                    )
                }),
            ast::StatementKind::Assignment(assignment) => {
                self.lower_assignment(assignment, statement.get_meta_data().clone())
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
                        statement.get_meta_data().clone()
                    ),
                ))
            }
        }
    }

    fn lower_assignment(
        &mut self,
        assignment: &ast::Assignment,
        meta: NodeMetaData
    ) -> Option<(NodeId, hir::Statement)> {
        let id = self.expect_node_id(assignment.node_id, meta.span)?;

        Some((
            id,
            hir::Statement::new_assign(
                hir::Assign {
                    id,
                    left: self.lower_expression(&assignment.left)?,
                    right: self.lower_expression(&assignment.right)?,
                },
                meta
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
            if variable.ty.get_modifier() == Some(TypeModifier::Mut) {
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

        self.push_local(variable.name.to_string(), id);

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
        self.module.functions.insert(id, signature.clone());

        let prev_scope = self.push_scope();
        
        for field in &signature.parameters {
            self.push_local(field.name.to_string(), field.id);
        }

        let block = self.lower_block(&function.block, span)?;
        self.pop_scope(prev_scope);

        let body_id = block.id;
        self.push_block(block, function.block.span);

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
            parameters: self.lower_parameter(&function_signature.parameters)?,
            vis: Visibility::from_name(&function_signature.name),
        })
    }

     fn lower_parameter(
        &mut self,
        types: &ast::NamedTupleType,
    ) -> Option<hir::NamedTupleType> {
        let mut tuple = hir::NamedTupleType::with_capacity(types.len());
        for (name, ty, id) in types {
            let id = self.expect_node_id(*id, ty.span)?;
            tuple.push(hir::FieldType::new(
                id,
                name.clone(),
                self.lower_type(ty)?,
                Visibility::Public,
            ));
        }

        Some(tuple)
    }
}
