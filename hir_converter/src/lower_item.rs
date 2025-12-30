use crate::HirLowerer;
use soul_hir::{self as hir, HirBlockId, HirType};
use soul_ast::abstract_syntax_tree::{self as ast, SoulType};

impl<'hir> HirLowerer<'hir> {
    
    pub(super) fn lower_global_statment(&mut self, statement: &ast::Statement) {
        match &statement.node {
            ast::StatementKind::Import(imports) => {
                let item = hir::Item::Import(hir::Import{
                    id: self.alloc_id(),
                    paths: imports.clone(),
                });
                self.add_item(item);
            }
            ast::StatementKind::UseBlock(use_block) => {
                let item = self.lower_use_block(use_block);
                self.add_item(item);
            }
            ast::StatementKind::Variable(variable) => {
                let item = self.lower_variable(variable);
                self.add_item(item);
            }
            ast::StatementKind::Function(function) => {
                let item = self.lower_function(function);
                self.add_item(item);
            }
            ast::StatementKind::Union(union) => {
                let item = self.lower_union(union);
                self.add_item(item);
            }
            ast::StatementKind::Class(class) => {
                let item = self.lower_class(class);
                self.add_item(item);
            }
            ast::StatementKind::Struct(r#struct) => {
                let item = self.lower_struct(r#struct);
                self.add_item(item);
            }
            ast::StatementKind::Trait(r#trait) => {
                let item = self.lower_trait(r#trait);
                self.add_item(item);
            }
            ast::StatementKind::Enum(r#enum) => {
                let item = self.lower_enum(r#enum);
                self.add_item(item);
            }

            ast::StatementKind::EndFile
            | ast::StatementKind::CloseBlock 
            | ast::StatementKind::Expression(_)
            | ast::StatementKind::Assignment(_) => {
                #[cfg(debug_assertions)]
                panic!("global scope can not be {:?}", statement.node);
                #[cfg(not(debug_assertions))]
                return
            }
        }
    }

    fn lower_function(&mut self, function: &ast::Function) -> hir::Item {
        let signature = &function.signature.node;
        let mut params = Vec::with_capacity(signature.parameters.types.len()); 
        
        for (name, ty, _) in &signature.parameters.types {

            params.push(hir::Parameter{ 
                id: self.alloc_id(), 
                name: name.clone(), 
                ty: self.lower_type(&ty) 
            });
        }

        hir::Item::Function(hir::Function {
            params,
            id: self.alloc_id(),
            body: self.lower_body(&function.block),
            name: signature.name.clone(),
            modifier: signature.modifier,
            return_type: self.lower_type(&signature.return_type),
            generics: self.lower_generic_declares(&signature.generics),
        })
    }
    
    fn lower_generic_declares(
        &mut self, 
        generics: &Vec<ast::GenericDeclare>,
    ) -> Vec<hir::GenericDeclare> {

        generics.iter()
            .map(|el| {
                match &el.kind {
                    ast::GenericDeclareKind::Lifetime(ident) => {
                        hir::GenericDeclare::Lifetime(ident.clone())
                    }
                    ast::GenericDeclareKind::Type { name, traits, default } => {
                        let hir_traits = traits.into_iter()
                            .map(|ty| self.lower_type(&ty))
                            .collect();
                        
                        let hir_default = default.as_ref().map(|ty| self.lower_type(ty));
                        hir::GenericDeclare::Type { 
                            name: name.clone(), 
                            traits: hir_traits, 
                            default: hir_default,
                        }
                    }
                    ast::GenericDeclareKind::Expression { name, for_type, default } => {
                        let hir_for_type = for_type.as_ref().map(|ty| self.lower_type(ty));
                        let hir_default = default.as_ref().map(|val| {
                            let expression = self.lower_expression(val);
                            self.add_expression(expression)
                        });

                        hir::GenericDeclare::Expression { 
                            name: name.clone(), 
                            default: hir_default,
                            for_type: hir_for_type, 
                        }
                    }
                }
            })
            .collect()
    }

    fn lower_type(&mut self, ty: &SoulType) -> HirType {
        let generics = ty.generics
            .iter()
            .map(|el| self.generics_define_to_type(el))
            .collect();
        
        HirType {
            kind: hir::HirTypeKind::None,
            modifier: ty.modifier,
            generics,
        }
    }
        
    fn lower_struct(&mut self, r#struct: &ast::Struct) -> hir::Item {
        let hir_struct = hir::Struct {
            id: self.alloc_id(),
            name: r#struct.name.clone(),
            fields: self.lower(),
            generics: self.lower_generic_declares(&r#struct.generics),
        };
    }

    fn lower_field(&mut self, field: &ast::Field) -> hir::
    
    fn lower_use_block(&mut self, use_block: &ast::UseBlock) -> hir::Item {
        todo!()
    }
    
    fn lower_variable(&mut self, variable: &ast::Variable) -> hir::Item {
        todo!()
    }
    
    fn lower_union(&mut self, union: &ast::Union) -> hir::Item {
        todo!()
    }
    
    fn lower_class(&mut self, class: &ast::Class) -> hir::Item {
        todo!()
    }
    
    fn lower_trait(&mut self, r#trait: &ast::Trait) -> hir::Item {
        todo!()
    }
    
    fn lower_enum(&mut self, r#enum: &ast::Enum) -> hir::Item {
        todo!()
    }

    fn lower_body(&mut self, block: &ast::Block) -> HirBlockId {
        todo!()
    }

    fn generics_define_to_type(&mut self, generics: &ast::GenericDefine) -> HirType {
        todo!()
    }

    fn lower_expression(&mut self, expression: &ast::Expression) -> hir::Expression {
        todo!()
    }
}