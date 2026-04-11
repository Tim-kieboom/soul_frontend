use ast::Literal;
use hir::{ComplexLiteral, ExpressionId, HirTree, LocalId, PlaceId, StructId, TypeId};
use typed_hir::{Field, Struct};

pub(crate) mod binary;
pub(crate) mod unary;
mod utils;
use soul_utils::{Ident, ids::IdAlloc, vec_map::VecMap};
use typed_hir::{ThirType, TypedHir};
pub(crate) use utils::*;

use crate::{binary::interpret_binary, unary::interpret_unary};

pub fn literal_resolve(hir: &HirTree, types: &TypedHir) -> VecMap<ExpressionId, ComplexLiteral> {
    let mut interpreter = LiteralInterpreter::new(hir, types);

    interpreter.collect_literals();
    interpreter.resolve_literals();
    interpreter.consume_to_literals()
}

struct LiteralInterpreter<'a> {
    hir: &'a HirTree,
    types: &'a TypedHir,
    locals: VecMap<LocalId, ExpressionId>,
    literals: VecMap<ExpressionId, ComplexLiteral>,
}
impl<'a> LiteralInterpreter<'a> {
    pub const fn new(hir: &'a HirTree, types: &'a TypedHir) -> Self {
        Self {
            hir,
            types,
            locals: VecMap::const_default(),
            literals: VecMap::const_default(),
        }
    }

    fn collect_literals(&mut self) {
        for (id, local_info) in self.hir.nodes.locals.entries() {
            let ty = self.get_type(self.types.types_table.locals[id]);

            if ty.is_mutable() {
                continue;
            }

            match &local_info.kind {
                hir::LocalKind::Temp(value) | hir::LocalKind::Variable(Some(value)) => {
                    _ = self.locals.insert(id, *value)
                }
                _ => (),
            }
        }
    }

    fn resolve_literals(&mut self) {
        for value_id in self.hir.nodes.expressions.keys() {
            if let Some(literal) = self.resolve_expression(value_id) {
                self.literals.insert(value_id, literal);
            }
        }
    }

    fn resolve_expression(&self, expression_id: hir::ExpressionId) -> Option<ComplexLiteral> {
        let value = &self.hir.nodes.expressions[expression_id];
        match &value.kind {
            hir::ExpressionKind::Null
            | hir::ExpressionKind::Error
            | hir::ExpressionKind::Block(_)
            | hir::ExpressionKind::DeRef(_)
            | hir::ExpressionKind::Sizeof(_)
            | hir::ExpressionKind::Literal(_)
            | hir::ExpressionKind::If { .. }
            | hir::ExpressionKind::Ref { .. }
            | hir::ExpressionKind::Function(_)
            | hir::ExpressionKind::Call { .. }
            | hir::ExpressionKind::Cast { .. }
            | hir::ExpressionKind::While { .. }
            | hir::ExpressionKind::InnerRawStackArray { .. } => None,

            hir::ExpressionKind::StructConstructor { ty, values, .. } => {
                self.interpret_struct_contructor(*ty, values, expression_id)
            }

            hir::ExpressionKind::Load(place) => self.interpret_place(*place),
            hir::ExpressionKind::Local(id) => self.interpret_local(*id),
            hir::ExpressionKind::Unary(unary) => {
                let value = self.try_get_literal(unary.expression)?;
                interpret_unary(&unary.operator, value.try_basic_ref()?).map(|l| l.to_complex())
            }
            hir::ExpressionKind::Binary(binary) => {
                let left = self.try_get_literal(binary.left)?;
                let right = self.try_get_literal(binary.right)?;
                interpret_binary(
                    left.try_basic_ref()?,
                    &binary.operator,
                    right.try_basic_ref()?,
                )
                .map(|l| l.to_complex())
            }
        }
    }

    fn interpret_struct_contructor(
        &self,
        struct_id: StructId,
        values: &Vec<(Ident, ExpressionId)>,
        expression_id: hir::ExpressionId,
    ) -> Option<ComplexLiteral> {
        let r#struct = self.types.types_map.id_to_struct(struct_id)?;

        let mut literals = Vec::new();

        let dummy = (ComplexLiteral::Basic(Literal::Bool(false)), TypeId::error());
        literals.resize(r#struct.fields.len(), dummy);

        let mut all_fields_const = true;
        for (name, value) in values {
            let ty = self.expression_type(*value);
            let literal = self.try_get_literal(*value)?;

            let i = match self.find_field_index(r#struct, name.as_str()) {
                Some(val) => val,
                None => continue,
            };

            let complex = literal.consume_to_complex();
            let field_type = r#struct.fields[i].ty;
            if self.get_type(field_type).is_mutable() || complex.is_mutable() {
                all_fields_const = false;
            }

            literals[i] = (complex, ty);
        }

        Some(ComplexLiteral::Struct {
            struct_id,
            struct_type: self.expression_type(expression_id),
            values: literals,
            all_fields_const,
        })
    }

    fn interpret_place(&self, place: PlaceId) -> Option<ComplexLiteral> {
        match &self.hir.nodes.places[place].kind {
            hir::PlaceKind::Temp(id) | hir::PlaceKind::Local(id) => self.interpret_local(*id),

            hir::PlaceKind::Deref(_)
            | hir::PlaceKind::Index { .. }
            | hir::PlaceKind::Field { .. } => None,
        }
    }

    fn interpret_local(&self, id: LocalId) -> Option<ComplexLiteral> {
        match self.locals.get(id) {
            Some(value_id) => self.literals.get(*value_id).cloned(),
            None => None,
        }
    }

    fn try_get_literal(&self, value_id: hir::ExpressionId) -> Option<LiteralRef<'a>> {
        if let hir::ExpressionKind::Literal(literal) = &self.hir.nodes.expressions[value_id].kind {
            return Some(LiteralRef::BasicRef(literal));
        }

        self.resolve_expression(value_id).map(LiteralRef::Owner)
    }

    fn get_type(&self, ty: TypeId) -> &ThirType {
        self.types
            .types_map
            .id_to_type(ty)
            .expect("should have type")
    }

    fn expression_type(&self, id: ExpressionId) -> TypeId {
        self.types
            .types_table
            .expressions
            .get(id)
            .copied()
            .unwrap_or(TypeId::error())
    }

    fn consume_to_literals(self) -> VecMap<ExpressionId, ComplexLiteral> {
        self.literals
    }

    fn find_field_index(&self, r#struct: &Struct, name: &str) -> Option<usize> {
        let field_name = |field: &Field| &self.hir.nodes.fields[field.id].name;

        r#struct
            .fields
            .iter()
            .enumerate()
            .find(|(_i, field)| field_name(field) == name)
            .map(|(i, _)| i)
    }
}

pub trait ToComplex {
    fn to_complex(self) -> ComplexLiteral;
}
impl ToComplex for Literal {
    fn to_complex(self) -> ComplexLiteral {
        ComplexLiteral::Basic(self)
    }
}

/// enum to avoid allow owner and ref to avoid .clone()
enum LiteralRef<'a> {
    Owner(ComplexLiteral),
    BasicRef(&'a Literal),
}
impl<'a> LiteralRef<'a> {
    fn consume_to_complex(self) -> ComplexLiteral {
        match self {
            LiteralRef::Owner(complex_literal) => complex_literal,
            LiteralRef::BasicRef(literal) => literal.clone().to_complex(),
        }
    }

    fn try_basic_ref(&self) -> Option<&Literal> {
        match self {
            LiteralRef::Owner(complex) => match complex {
                ComplexLiteral::Basic(literal) => Some(literal),
                ComplexLiteral::Struct { .. } => None,
            },
            LiteralRef::BasicRef(literal) => Some(literal),
        }
    }
}
