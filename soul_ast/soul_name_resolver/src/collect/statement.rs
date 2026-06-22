
use ast_model::{CustomType, statements::{StatementId, StatementKind}};
use soul_utils::soul_error_internal;

use crate::NameResolver;

impl<'a> NameResolver<'a> {
    
    pub(super) fn collect_statement(&mut self, id: StatementId) {
        
        let Some(statement) = self.store.statements.get(id) else {
            self.log_fault(soul_error_internal!(format!("{id:?} not found"), None));
            return
        };

        match &statement.node {
            StatementKind::Enum(enum_) => {
                self.declare_enum(id, enum_);
                if self.current.in_global {
                    let ty = CustomType::Enum(enum_.clone());
                    self.header_insert_custom_type(id, ty);
                }
            }
            StatementKind::Trait(trait_) => {
                self.declare_trait(id, trait_);
                if self.current.in_global {
                    let ty = CustomType::Trait(trait_.clone());
                    self.header_insert_custom_type(id, ty);
                }
            }
            StatementKind::Struct(struct_) => {
                self.declare_struct(id, struct_);
                if self.current.in_global {
                    let ty = CustomType::Struct(struct_.clone());
                    self.header_insert_custom_type(id, ty);
                }
            }
            StatementKind::Import(import) => {
                let span = statement.span;
                for path in &import.paths {
                    self.collect_import_path(path, span);
                }
            }
            StatementKind::TypeDef(type_def) => todo!(),
            StatementKind::Variable(variable) => todo!(),
            StatementKind::UseBlock(use_block) => todo!(),
            StatementKind::Function(function_id) => todo!(),
            StatementKind::Assignment(assignment) => todo!(),
            StatementKind::ExternalFunction(function_id) => todo!(),
            StatementKind::Expression { expression, ends_semicolon } => todo!(),
        }
    }
}