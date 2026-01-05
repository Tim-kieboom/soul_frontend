use crate::HirLowerer;
use soul_ast::abstract_syntax_tree::{self as ast, Spanned, Visibility};
use soul_hir::{self as hir, HirBlockId, HirId};
#[allow(deprecated)]
use soul_utils::remind_warning;
use soul_utils::{SoulError, SoulErrorKind, SoulPagePath, Span, VecMap};

impl<'hir> HirLowerer<'hir> {
    pub(super) fn lower_global_statment(&mut self, statement: &ast::Statement) {
        match &statement.node {
            ast::StatementKind::Import(i) => {
                self.lower_import(i);
            }
            ast::StatementKind::Function(f) => {
                self.lower_function(f);
            }
            ast::StatementKind::Struct(s) => {
                self.lower_struct(s);
            }
            ast::StatementKind::Trait(t) => {
                self.lower_trait(t);
            }
            ast::StatementKind::Enum(e) => {
                self.lower_enum(e);
            }
            ast::StatementKind::Union(u) => {
                self.lower_union(u);
            }
            ast::StatementKind::UseBlock(u) => {
                self.lower_use_block(u);
            }
            other => {
                self.log_error(SoulError::new(
                    format!(
                        "{} is not allowed as global statement",
                        other.get_variant_name()
                    ),
                    SoulErrorKind::InvalidContext,
                    Some(statement.span),
                ));
            }
        }
    }

    fn lower_import(&mut self, imports: &Vec<SoulPagePath>) -> hir::Item {
        hir::Item::Import(hir::Import {
            id: self.alloc_id(),
            paths: imports.clone(),
        })
    }

    fn lower_function(&mut self, function: &ast::Function) -> Option<()> {
        let ast_signature = &function.signature.node;

        let ast_params = &ast_signature.parameters.types;
        let mut parameters = Vec::with_capacity(ast_params.len());
        for (name, ty, _id) in ast_params {
            parameters.push(hir::Parameter {
                id: self.alloc_id(),
                name: name.clone(),
                ty: self.get_hir_type(ty)?,
            });
        }

        let callee = match &ast_signature.callee {
            Some(val) => Some(Spanned::with_atribute(
                hir::FunctionCallee {
                    ty: self.get_hir_type(&val.node.extention_type)?,
                    this: val.node.this,
                },
                val.span,
                val.attributes.clone(),
            )),
            None => None,
        };

        let body = self.lower_block(&function.block, vec![]);
        let signature = hir::FunctionSignature {
            callee,
            parameters,
            name: ast_signature.name.clone(),
            modifier: ast_signature.modifier,
            vis: self.vis_from_name(&ast_signature.name),
            return_type: self.get_hir_type(&ast_signature.return_type)?,
            generics: self.get_hir_generic_declares(&ast_signature.generics)?,
        };

        let item = hir::Item::Function(hir::Function {
            body,
            signature,
            id: self.alloc_id(),
        });

        self.add_item(item);
        Some(())
    }

    fn get_hir_generic_declares(
        &mut self,
        generics: &Vec<ast::GenericDeclare>,
    ) -> Option<Vec<hir::GenericDeclare>> {
        let mut hir_generics = Vec::with_capacity(generics.len());
        for el in generics {
            let generic = match &el.kind {
                ast::GenericDeclareKind::Lifetime(ident) => {
                    hir::GenericDeclare::Lifetime(ident.clone())
                }
                ast::GenericDeclareKind::Type {
                    name,
                    traits,
                    default,
                } => {
                    let mut hir_traits = Vec::with_capacity(traits.len());
                    for ty in traits {
                        hir_traits.push(self.get_hir_type(&ty)?);
                    }

                    let hir_default = match default {
                        Some(ty) => Some(self.get_hir_type(ty)?),
                        None => None,
                    };
                    hir::GenericDeclare::Type {
                        name: name.clone(),
                        traits: hir_traits,
                        default: hir_default,
                    }
                }
                ast::GenericDeclareKind::Expression {
                    name,
                    for_type,
                    default,
                } => {
                    let hir_for_type = match for_type {
                        Some(ty) => Some(self.get_hir_type(ty)?),
                        None => None,
                    };
                    let hir_default = match &default {
                        Some(val) => Some(self.lower_expression(val)?),
                        None => None,
                    };

                    hir::GenericDeclare::Expression {
                        name: name.clone(),
                        default: hir_default,
                        for_type: hir_for_type,
                    }
                }
            };

            hir_generics.push(generic);
        }

        Some(hir_generics)
    }

    fn lower_struct(&mut self, r#struct: &ast::Struct) -> Option<hir::Item> {
        let fields = r#struct
            .fields
            .iter()
            .filter_map(|field| self.lower_field(&field.node))
            .collect();

        remind_warning("add private contructor");

        Some(hir::Item::Struct(hir::Struct {
            fields,
            id: self.alloc_id(),
            name: r#struct.name.clone(),
            vis: self.vis_from_name(&r#struct.name),
            generics: self.get_hir_generic_declares(&r#struct.generics)?,
        }))
    }

    fn lower_field(&mut self, field: &ast::Field) -> Option<HirId> {
        let value = match &field.default_value {
            Some(val) => Some(self.lower_expression(val)?),
            None => None,
        };

        let hir_field = hir::Field {
            value,
            access: field.vis,
            id: self.alloc_id(),
            name: field.name.clone(),
            ty: self.get_hir_type(&field.ty)?,
        };

        let span = hir_field.name.span;
        let statement = hir::Statement::new(hir::StatementKind::Field(hir_field), span);

        match self.add_statement(statement) {
            Ok(val) => Some(val),
            Err(err) => {
                self.log_error(err);
                None
            }
        }
    }

    fn lower_use_block(&mut self, use_block: &ast::UseBlock) {
        todo!()
    }

    fn lower_variable(&mut self, variable: &ast::Variable) {
        todo!()
    }

    fn lower_union(&mut self, union: &ast::Union) {
        todo!()
    }

    fn lower_class(&mut self, class: &ast::Class) {
        remind_warning("add public constructor");
        todo!()
    }

    fn lower_trait(&mut self, r#trait: &ast::Trait) {
        todo!()
    }

    fn lower_enum(&mut self, r#enum: &ast::Enum) {
        todo!()
    }

    fn lower_contructor(&mut self, vis: Visibility, span: Span) {}

    pub(super) fn lower_block(&mut self, block: &ast::Block, add_statments: Vec<hir::Statement>) -> HirBlockId {
        let scope = self.push_scope();

        let mut statements = VecMap::from_vec(
            block
                .statements
                .iter()
                .map(|el| (self.alloc_id(), self.try_get_hir_statment(el)))
                .collect(),
        );

        statements.extend(
            add_statments
                .into_iter()
                .map(|el| (el.node.get_id(), el))
        );

        let block = hir::Block {
            statements,
            scope_id: scope,
            id: self.alloc_id(),
            modifier: block.modifier,
        };

        let body_id = self.alloc_id();
        self.module.bodies.insert(body_id, hir::Body::Block(block));
        self.current_body_id = Some(body_id);
        body_id
    }

    fn try_get_hir_statment(&mut self, statement: &ast::Statement) -> Option<hir::Statement> {
        let kind = match &statement.node {
            ast::StatementKind::Assignment(assignment) => hir::StatementKind::Assign(hir::Assign {
                id: self.alloc_id(),
                left: self.lower_expression(&assignment.left)?,
                right: self.lower_expression(&assignment.right)?,
            }),
            ast::StatementKind::Expression(expression) => {
                hir::StatementKind::Expression(hir::StatementExpression {
                    id: self.alloc_id(),
                    expression: self.lower_expression(expression)?,
                })
            }
            ast::StatementKind::Variable(variable) => {
                let value = match &variable.initialize_value {
                    Some(val) => Some(self.lower_expression(val)?),
                    None => None,
                };
                hir::StatementKind::Variable(Box::new(hir::Variable {
                    value,
                    id: self.alloc_id(),
                    name: variable.name.clone(),
                    ty: self.get_hir_type(&variable.ty)?,
                    vis: self.vis_from_name(&variable.name),
                }))
            }
            ast::StatementKind::UseBlock(use_block) => {
                self.lower_use_block(use_block);
                return None;
            }
            ast::StatementKind::Function(function) => {
                self.lower_function(function);
                return None;
            }
            ast::StatementKind::Struct(r#struct) => {
                self.lower_struct(r#struct);
                return None;
            }
            ast::StatementKind::Import(imports) => {
                self.lower_import(imports);
                return None;
            }
            ast::StatementKind::Trait(r#trait) => {
                self.lower_trait(r#trait);
                return None;
            }
            ast::StatementKind::Class(class) => {
                self.lower_class(class);
                return None;
            }
            ast::StatementKind::Union(union) => {
                self.lower_union(union);
                return None;
            }
            ast::StatementKind::Enum(r#enum) => {
                self.lower_enum(r#enum);
                return None;
            }

            ast::StatementKind::EndFile | ast::StatementKind::CloseBlock => return None,
        };

        Some(hir::Statement::new(kind, statement.span))
    }
}
