use crate::{
    SementicFault,
    steps::sementic_analyser::{SementicInfo, SementicPass},
};
use models::{
    abstract_syntax_tree::{
        AbstractSyntaxTree,
        block::Block,
        soul_type::{GenericKind, SoulType, TypeKind},
        statment::Ident,
    },
    error::{SoulError, SoulErrorKind, Span},
    sementic_models::scope::{NodeId, ScopeTypeEntry, ScopeTypeEntryKind},
    soul_names::StackArrayKind,
};

pub struct TypeResolver<'a> {
    pub info: &'a mut SementicInfo,
}

impl<'a> SementicPass<'a> for TypeResolver<'a> {
    fn new(info: &'a mut SementicInfo) -> Self {
        Self { info }
    }

    fn run(&mut self, ast: &mut AbstractSyntaxTree) {
        self.resolve_block(&mut ast.root);
    }
}

impl<'a> TypeResolver<'a> {
    pub(super) fn resolve_block(&mut self, block: &mut Block) {
        for statment in &mut block.statments {
            self.resolve_statement(statment);
        }
    }

    fn lookup_type(&self, ident: &Ident) -> Option<ScopeTypeEntry> {
        self.info.scopes.lookup_type(ident)
    }

    pub(super) fn resolve_type(&mut self, ty: &mut SoulType) {
        let span = ty.span;

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
                        format!("type '{}' not found", ident.as_str()),
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
                self.resolve_type(&mut array_type.of_type);
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
                    self.resolve_type(ty);
                }
            }
            TypeKind::Pointer(soul_type) => self.resolve_type(soul_type),
            TypeKind::Optional(soul_type) => self.resolve_type(soul_type),
            TypeKind::Function(function_type) => {
                for item in &mut function_type.parameters.types {
                    self.resolve_type(item);
                }
                self.resolve_type(&mut function_type.return_type);
            }
            TypeKind::Reference(reference_type) => {
                self.resolve_type(&mut reference_type.inner)
            }
            TypeKind::NamedTuple(named_tuple_type) => {
                for (_name, ty, _node_id) in &mut named_tuple_type.types {
                    self.resolve_type(ty);
                }
            }

            TypeKind::None
            | TypeKind::Type
            | TypeKind::Primitive(_)
            | TypeKind::InternalComplex(_) => (),
        }
    }

    pub(super) fn log_error(&mut self, err: SoulError) {
        self.info.faults.push(SementicFault::error(err));
    }

    fn lookup_variable(&self, ident: &Ident) -> Option<NodeId> {
        self.info.scopes.lookup_variable(ident)
    }

    pub(super) fn resolve_variable(
        &mut self,
        ident: &Ident,
        resolved: &mut Option<NodeId>,
        span: Span,
    ) {
        match self.lookup_variable(ident) {
            Some(id) => *resolved = Some(id),
            None => self.log_error(SoulError::new(
                format!("variable '{}' is undefined in scope", ident.as_str()),
                SoulErrorKind::ScopeError,
                Some(span),
            )),
        }
    }
}
