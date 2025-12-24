use models::{
    abstract_syntax_tree::soul_type::{GenericDeclare, GenericKind, SoulType, TypeKind},
    error::{SoulError, SoulErrorKind, Span},
    sementic_models::scope::{ScopeTypeEntryKind, ScopeTypeKind},
    soul_names::StackArrayKind,
};

use crate::steps::sementic_analyser::name_resolution::name_resolver::NameResolver;

impl NameResolver {
    pub(crate) fn resolve_type(&mut self, ty: &mut SoulType, span: Span) {
        match &mut ty.kind {
            TypeKind::Stub { ident, .. } => {
                if let Some(entry) = self.lookup_type(ident) {
                    match entry.kind {
                        ScopeTypeEntryKind::Struct => {
                            *ty = SoulType::new(ty.modifier, TypeKind::Struct(entry.node_id), span)
                        }
                        ScopeTypeEntryKind::Class => {
                            *ty = SoulType::new(ty.modifier, TypeKind::Class(entry.node_id), span)
                        }
                        ScopeTypeEntryKind::Trait => {
                            *ty = SoulType::new(ty.modifier, TypeKind::Trait(entry.node_id), span)
                        }
                        ScopeTypeEntryKind::Union => {
                            *ty = SoulType::new(ty.modifier, TypeKind::Union(entry.node_id), span)
                        }
                        ScopeTypeEntryKind::Enum => {
                            *ty = SoulType::new(ty.modifier, TypeKind::Enum(entry.node_id), span)
                        }
                        ScopeTypeEntryKind::LifeTime => {
                            *ty = SoulType::new(
                                ty.modifier,
                                TypeKind::Generic {
                                    node_id: entry.node_id,
                                    kind: GenericKind::LifeTime,
                                },
                                span,
                            )
                        }
                        ScopeTypeEntryKind::GenericType => {
                            *ty = SoulType::new(
                                ty.modifier,
                                TypeKind::Generic {
                                    node_id: entry.node_id,
                                    kind: GenericKind::Type,
                                },
                                span,
                            )
                        }
                        ScopeTypeEntryKind::GenericExpression => {
                            *ty = SoulType::new(
                                ty.modifier,
                                TypeKind::Generic {
                                    node_id: entry.node_id,
                                    kind: GenericKind::Expression,
                                },
                                span,
                            )
                        }
                    }
                } else {
                    self.log_error(SoulError::new(
                        format!("type '{ident}' not found"),
                        SoulErrorKind::ScopeError,
                        Some(ty.span),
                    ));
                }
            }
            TypeKind::Generic { .. }
            | TypeKind::Enum(_)
            | TypeKind::Class(_)
            | TypeKind::Trait(_)
            | TypeKind::Union(_)
            | TypeKind::Struct(_) => (),

            TypeKind::Array(array_type) => {
                self.resolve_type(&mut array_type.of_type, span);
                match &mut array_type.size {
                    Some(StackArrayKind::Number(_)) => (),
                    Some(StackArrayKind::Ident { ident, resolved }) => {
                        if let Some(entry) = self.lookup_type(ident) {
                            *resolved = Some(entry.node_id);
                        } else {
                            self.resolve_variable(ident, resolved, ty.span);
                        }
                    }
                    None => (),
                }
            }
            TypeKind::Tuple(tuple_type) => {
                for ty in &mut tuple_type.types {
                    self.resolve_type(ty, span);
                }
            }
            TypeKind::Pointer(soul_type) => self.resolve_type(soul_type, span),
            TypeKind::Optional(soul_type) => self.resolve_type(soul_type, span),
            TypeKind::Function(function_type) => {
                for item in &mut function_type.parameters.types {
                    self.resolve_type(item, span);
                }
                self.resolve_type(&mut function_type.return_type, span);
            }
            TypeKind::Reference(reference_type) => {
                self.resolve_type(&mut reference_type.inner, span)
            }
            TypeKind::NamedTuple(named_tuple_type) => {
                for (_name, ty, _node_id) in &mut named_tuple_type.types {
                    self.resolve_type(ty, span);
                }
            }

            TypeKind::None
            | TypeKind::Type
            | TypeKind::Primitive(_)
            | TypeKind::InternalComplex(_) => (),
        }
    }

    pub(super) fn resolve_generic_declares(&mut self, generics: &mut Vec<GenericDeclare>) {
        for generic in generics {
            let span = generic.span;
            self.declare_type(ScopeTypeKind::GenricDeclare(generic), span);
        }
    }
}
