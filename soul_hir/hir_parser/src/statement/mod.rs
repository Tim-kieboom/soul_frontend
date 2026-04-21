use ast::UseBlock;
use hir::{Assign, StatementId};
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    soul_names::KeyWord,
    span::ModuleId,
};

use crate::HirContext;
mod block;
mod function;

impl<'a> HirContext<'a> {
    pub fn lower_global(&mut self, module_id: ModuleId, global: &ast::Statement) {
        let kind = match &global.node {
            ast::StatementKind::UseBlock(UseBlock {
                use_type: _,
                generics,
                impls,
                methodes,
            }) => {
                if !generics.is_empty() {
                    todo!()
                }

                for methode in methodes {
                    let kind = hir::GlobalKind::Function(self.lower_function(methode));
                    let id = self.alloc_statement(&global.meta_data, global.span);
                    self.insert_global(module_id, hir::Global::new(kind, id));
                }

                if !impls.is_empty() {
                    todo!()
                }
                return;
            }
            ast::StatementKind::Import(_) => {
                return;
            }
            ast::StatementKind::Struct(_) => return, // gets lowered from DeclareStore
            ast::StatementKind::Variable(variable) => {
                hir::GlobalKind::Variable(self.lower_variable(variable))
            }
            ast::StatementKind::Function(function)
            | ast::StatementKind::ExternalFunction(function) => {
                hir::GlobalKind::Function(self.lower_function(function))
            }

            ast::StatementKind::Assignment(_) | ast::StatementKind::Expression { .. } => {
                self.log_error(SoulError::new(
                    format!(
                        "{} statement is not allowed as a global",
                        global.node.variant_name()
                    ),
                    SoulErrorKind::InvalidContext,
                    Some(global.span),
                ));
                return;
            }
        };

        let id = self.alloc_statement(&global.meta_data, global.span);
        self.insert_global(module_id, hir::Global::new(kind, id));
    }

    pub fn lower_statement(
        &mut self,
        module_id: ModuleId,
        global: &ast::Statement,
    ) -> Option<hir::Statement> {
        let kind = match &global.node {
            ast::StatementKind::UseBlock(UseBlock {
                use_type: _,
                generics,
                impls,
                methodes,
            }) => {
                if !generics.is_empty() {
                    todo!()
                }

                for methode in methodes {
                    let kind = hir::GlobalKind::Function(self.lower_function(methode));
                    let id = self.alloc_statement(&global.meta_data, global.span);
                    self.insert_global(module_id, hir::Global::new(kind, id));
                }

                if !impls.is_empty() {
                    todo!()
                }
                return None;
            }
            ast::StatementKind::Import(_) => {
                return None;
            }
            ast::StatementKind::Struct(object) => {
                self.lower_struct(object);
                return None;
            }
            ast::StatementKind::Variable(variable) => {
                hir::StatementKind::Variable(self.lower_variable(variable))
            }
            ast::StatementKind::Function(function)
            | ast::StatementKind::ExternalFunction(function) => {
                let id = self.alloc_statement(&global.meta_data, global.span);

                let hir_function = self.lower_function(function);
                let kind = hir::GlobalKind::Function(hir_function);
                self.insert_global(module_id, hir::Global::new(kind, id));
                return None;
            }
            ast::StatementKind::Assignment(assignment) => hir::StatementKind::Assign(Assign {
                place: self.lower_place(&assignment.left),
                value: self.lower_expression(&assignment.right),
            }),
            ast::StatementKind::Expression {
                id: _,
                expression,
                ends_semicolon,
            } => {
                use ast::ExpressionKind::ReturnLike;

                if let ReturnLike(return_like) = &expression.node {
                    self.lower_return_like(return_like)
                } else {
                    hir::StatementKind::Expression {
                        value: self.lower_expression(expression),
                        ends_semicolon: *ends_semicolon,
                    }
                }
            }
        };

        let id = self.alloc_statement(&global.meta_data, global.span);
        Some(hir::Statement::new(kind, id))
    }

    fn lower_variable(&mut self, variable: &ast::Variable) -> hir::Variable {
        let ty = match &variable.ty {
            ast::VarTypeKind::NonInveredType(soul_type) => {
                self.lower_type(soul_type, variable.name.span)
            }
            ast::VarTypeKind::InveredType(type_modifier) => {
                self.new_infer_type(vec![], Some(*type_modifier), variable.name.span)
            }
        };

        let value = variable
            .initialize_value
            .as_ref()
            .map(|val| self.lower_expression(val));

        let local = self.id_generator.alloc_local();
        self.insert_variable(&variable.name, local, ty, value);

        if let Some(node_id) = variable.node_id {
            self.node_id_to_local.insert(node_id, local);
        }

        hir::Variable { local }
    }

    pub(crate) fn lower_struct(&mut self, object: &ast::Struct) {
        let name = object.name.clone();

        let mut generics = vec![];
        for generic in &object.generics {
            let id = self.insert_generic(generic.name.to_string());
            generics.push(id);
        }

        let struct_id = self.tree.info.types.alloc_struct();

        let mut fields = vec![];
        for field in &object.fields {
            let ty = self.lower_type(&field.ty, field.name.span);
            let id = self.id_generator.alloc_field();

            let hir_field = hir::Field {
                id,
                ty,
                struct_id,
                name: field.name.to_string(),
            };

            fields.push(hir_field.clone());
            self.tree.nodes.fields.insert(id, hir_field);
        }

        self.insert_struct(struct_id, hir::Struct { name, fields });
    }

    pub(crate) fn lower_return_like(
        &mut self,
        return_like: &ast::ReturnLike,
    ) -> hir::StatementKind {
        let value = return_like
            .value
            .as_ref()
            .map(|val| self.lower_expression(val));

        if matches!(
            return_like.kind,
            ast::ReturnKind::Break | ast::ReturnKind::Continue
        ) && let Some(value) = &return_like.value
        {
            self.log_error(SoulError::new(
                format!("{} can not contain expression", KeyWord::Continue.as_str()),
                SoulErrorKind::InvalidContext,
                Some(value.span),
            ));
        }

        match return_like.kind {
            ast::ReturnKind::Return => hir::StatementKind::Return(value),
            ast::ReturnKind::Continue => hir::StatementKind::Continue,
            ast::ReturnKind::Break => hir::StatementKind::Break,
        }
    }

    fn insert_global(&mut self, module_id: ModuleId, global: hir::Global) -> StatementId {
        let id = global.id;
        self.tree.nodes.modules[module_id].globals.push(global);
        id
    }
}
