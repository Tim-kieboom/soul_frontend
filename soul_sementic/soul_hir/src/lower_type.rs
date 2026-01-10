use std::vec;

use crate::HirLowerer;
use hir_model::{self as hir, HirType, HirTypeKind, LocalDefId, PrimitiveSize, Visibility};
use parser_models::ast::{self, SoulType};
use soul_utils::{
    Ident,
    soul_names::{InternalPrimitiveTypes, TypeModifier}, span::Span,
};

impl HirLowerer {
    pub(crate) fn lower_type(&mut self, ty: &SoulType) -> Option<HirType> {
        let kind = match &ty.kind {
            ast::TypeKind::None => hir::HirTypeKind::None,
            ast::TypeKind::Type => hir::HirTypeKind::Type,
            ast::TypeKind::Stub { ident: _, resolved } => {
                hir::HirTypeKind::Stub(self.expect_node_id(*resolved, ty.span)?)
            }
            ast::TypeKind::Primitive(prim) => hir::HirTypeKind::Primitive(to_primitive(*prim)),
            ast::TypeKind::Array(array_type) => match array_type.size {
                Some(size) => hir::HirTypeKind::StackArray {
                    ty: Box::new(self.lower_type(array_type.of_type.as_ref())?),
                    size,
                },
                None => hir::HirTypeKind::NamedTuple(self.desugar_array(&array_type.of_type)?),
            },
            ast::TypeKind::Tuple(soul_types) => {
                let mut types = hir::TupleType::with_capacity(soul_types.len());
                for ty in soul_types {
                    types.push(self.lower_type(ty)?);
                }
                hir::HirTypeKind::Tuple(types)
            }
            ast::TypeKind::NamedTuple(items) => {
                let mut types = hir::NamedTupleType::with_capacity(items.len());
                for (name, ty, id) in items {
                    let id = self.expect_node_id(*id, name.span)?;
                    types.push(hir::FieldType::new(
                        name.clone(),
                        self.lower_type(ty)?,
                        id,
                        Visibility::Public,
                    ));
                }
                hir::HirTypeKind::NamedTuple(types)
            }
            ast::TypeKind::Generic { node_id, kind:_ } => {
                let id = self.expect_node_id(*node_id, ty.span)?;
                hir::HirTypeKind::Generic(LocalDefId {
                    owner: id,
                    local_id: 0,
                })
            }
            ast::TypeKind::Reference(reference_type) => {
                let ty = Box::new(self.lower_type(&reference_type.inner)?);
                hir::HirTypeKind::Ref {
                    ty,
                    mutable: reference_type.mutable,
                }
            }
            ast::TypeKind::Pointer(soul_type) => {
                hir::HirTypeKind::Pointer(Box::new(self.lower_type(soul_type)?))
            }
            ast::TypeKind::Optional(soul_type) => {
                hir::HirTypeKind::NamedTuple(self.desugar_opional(soul_type)?)
            }
        };

        Some(HirType {
            kind,
            generics: self.lower_generic_define(&ty.generics)?,
            modifier: ty.modifier,
            span: ty.span,
        })
    }

    pub(crate) fn lower_named_tuple_type(
        &mut self,
        types: &ast::NamedTupleType,
    ) -> Option<hir::NamedTupleType> {
        let mut tuple = hir::NamedTupleType::with_capacity(types.len());
        for (name, ty, id) in types {
            let id = self.expect_node_id(*id, ty.span)?;
            tuple.push(hir::FieldType::new(
                name.clone(),
                self.lower_type(ty)?,
                id,
                Visibility::Public,
            ));
        }

        Some(tuple)
    }

    pub(crate) fn lower_generic_declare(
        &mut self,
        generics: &Vec<ast::GenericDeclare>,
    ) -> Option<Vec<hir::GenericDeclare>> {
        if generics.is_empty() {
            return Some(vec![])
        }

        todo!("impl generic declare")
    }

    pub(crate) fn lower_generic_define(
        &mut self,
        generics: &Vec<ast::GenericDefine>,
    ) -> Option<Vec<hir::GenericDefine>> {
        if generics.is_empty() {
            return Some(vec![])
        }

        todo!("impl generic define")
    }

    fn desugar_array(&mut self, inner_type: &SoulType) -> Option<hir::NamedTupleType> {
        let ty = self.lower_type(inner_type)?;
        let span = ty.span;
        
        let ptr = hir::FieldType::new(
            Ident::new("ptr".to_string(), span),
            to_pointer(ty, TypeModifier::Mut),
            self.id_generator.alloc(),
            Visibility::Private,
        );
        let len = hir::FieldType::new(
            Ident::new("len".to_string(), span), 
            new_uint(span), 
            self.id_generator.alloc(), 
            Visibility::Private,
        );
        let cap = hir::FieldType::new(
            Ident::new("cap".to_string(), span), 
            new_uint(span), 
            self.id_generator.alloc(), 
            Visibility::Private,
        );

        Some(hir::NamedTupleType::from([ptr,len,cap]))
    }

    fn desugar_opional(&mut self, inner_type: &SoulType) -> Option<hir::NamedTupleType> {
        let ty = self.lower_type(inner_type)?;
        let span = ty.span;
        
        let inner = hir::FieldType::new(
            Ident::new("inner".to_string(), span),
            ty,
            self.id_generator.alloc(),
            Visibility::Private,
        );
        let is_null  = hir::FieldType::new(
            Ident::new("isNull".to_string(), span),
            new_bool(span),
            self.id_generator.alloc(),
            Visibility::Private,
        );

        Some(hir::NamedTupleType::from([inner, is_null]))
    }
}

fn new_uint(span: Span) -> HirType {
    new_primitive(
        to_primitive(InternalPrimitiveTypes::Uint), 
        span, TypeModifier::Mut,
    )
}
fn new_bool(span: Span) -> HirType {
    new_primitive(
        to_primitive(InternalPrimitiveTypes::Boolean), 
        span, TypeModifier::Mut,
    )
}
fn new_primitive(prim: hir_model::Primitive, span: Span, modifier: TypeModifier) -> HirType {
    HirType { kind: HirTypeKind::Primitive(prim), generics: vec![], modifier, span }
}

fn to_pointer(ty: HirType, modifier: TypeModifier) -> HirType {
    let span = ty.span;
    HirType { kind: hir::HirTypeKind::Pointer(Box::new(ty)), generics: vec![], modifier, span }
}

fn to_primitive(prim: InternalPrimitiveTypes) -> hir::Primitive {
    use hir::Primitive;
    const NIL: PrimitiveSize = PrimitiveSize::Nil;
    const BIT8: PrimitiveSize = PrimitiveSize::Bit8;
    const BIT16: PrimitiveSize = PrimitiveSize::Bit16;
    const BIT32: PrimitiveSize = PrimitiveSize::Bit32;
    const BIT64: PrimitiveSize = PrimitiveSize::Bit64;
    const BIT124: PrimitiveSize = PrimitiveSize::Bit124;

    const DEFAUL: PrimitiveSize = BIT32;

    match prim {
        InternalPrimitiveTypes::None => Primitive::Uint(NIL),
        InternalPrimitiveTypes::Char => Primitive::Char(BIT8),
        InternalPrimitiveTypes::Char8 => Primitive::Char(BIT8),
        InternalPrimitiveTypes::Char16 => Primitive::Char(BIT16),
        InternalPrimitiveTypes::Char32 => Primitive::Char(BIT32),
        InternalPrimitiveTypes::Char64 => Primitive::Char(BIT64),
        InternalPrimitiveTypes::Boolean => Primitive::Boolean,
        InternalPrimitiveTypes::UntypedInt => Primitive::Int(DEFAUL),
        InternalPrimitiveTypes::Int => Primitive::Int(DEFAUL),
        InternalPrimitiveTypes::Int8 => Primitive::Int(BIT8),
        InternalPrimitiveTypes::Int16 => Primitive::Int(BIT16),
        InternalPrimitiveTypes::Int32 => Primitive::Int(BIT32),
        InternalPrimitiveTypes::Int64 => Primitive::Int(BIT64),
        InternalPrimitiveTypes::Int128 => Primitive::Int(BIT124),
        InternalPrimitiveTypes::UntypedUint => Primitive::Uint(DEFAUL),
        InternalPrimitiveTypes::Uint => Primitive::Uint(DEFAUL),
        InternalPrimitiveTypes::Uint8 => Primitive::Uint(BIT8),
        InternalPrimitiveTypes::Uint16 => Primitive::Uint(BIT16),
        InternalPrimitiveTypes::Uint32 => Primitive::Uint(BIT32),
        InternalPrimitiveTypes::Uint64 => Primitive::Uint(BIT64),
        InternalPrimitiveTypes::Uint128 => Primitive::Uint(BIT124),
        InternalPrimitiveTypes::UntypedFloat => Primitive::Float(DEFAUL),
        InternalPrimitiveTypes::Float16 => Primitive::Float(BIT16),
        InternalPrimitiveTypes::Float32 => Primitive::Float(BIT32),
        InternalPrimitiveTypes::Float64 => Primitive::Float(BIT64),
    }
}
