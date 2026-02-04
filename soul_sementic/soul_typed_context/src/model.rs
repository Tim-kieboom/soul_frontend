use hir_model::HirType;
use parser_models::scope::NodeId;
use soul_utils::span::Span;
use soul_utils::vec_map::{VecMap, VecMapIndex};
/// A type variable used during type inference in Hindley-Milner style type checking.
///
/// In your typed HIR type system, `TypeVariable` represents an **unknown type**
/// that the inference algorithm will later solve to a concrete `HirType`.
///
/// ## Why Type Variables?
///
/// Consider this program:
/// ```soul
/// x := 42;
/// y := x + 1;  // What type is `y`?
/// ```
///
/// The type checker doesn't know `x`'s type initially. It creates `TypeVariable(0)`
/// for `x`, then infers `TypeVariable(0)` must be `i32` (from `42`), substitutes
/// it, and resolves `y` to `i32`.
///
/// ## How it works:
/// 1. `fresh_var()` creates `TypeVariable(n)` where `n` is unique
/// 2. During inference, variables get unified: `TypeVariable(0) = i32`
/// 3. `resolve()` follows the substitution chain to find concrete types
///
/// ## Example flow:
/// ```soul
/// // Initial state
/// x: TypeVariable(0)  
/// // After seeing `42`
/// substitutions: { TypeVariable(0) → Known(i32) }
/// // resolve(TypeVariable(0)) returns Known(i32)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct TypeVariable(u32);
impl VecMapIndex for TypeVariable {
    fn new_index(value: usize) -> Self {
        TypeVariable(value as u32)
    }

    fn index(&self) -> usize {
        self.0 as usize
    }
}

/// A type used during inference.
///
/// `InferType` either contains a fully known `HirType` or a
/// type variable that will be solved later by unification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum InferType {
    Known(HirType),
    Variable(TypeVariable, Span),
}
impl InferType {
    pub fn contains_variable(&self, variable: TypeVariable) -> bool {
        match self {
            InferType::Known(_) => false,
            InferType::Variable(type_variable, _) => *type_variable == variable,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum Place {
    Local(NodeId, InferType),
}
impl Place {
    pub fn get_id(&self) -> NodeId {
        match self {
            Place::Local(node_id, _) => *node_id,
        }
    }
    pub fn get_type(&self) -> &InferType {
        match self {
            Place::Local(_, infer_type) => infer_type,
        }
    }
}

/// Global state for type inference.
///
/// This environment:
/// - Generates fresh type variables.
/// - Stores substitutions from type variables to inferred types.
/// - Can resolve a possibly-variable type to its most concrete form.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct TypeEnvironment {
    /// Counter used to allocate fresh `TypeVariable`s.
    next_var: u32,

    /// Current substitutions from type variables to inferred types.
    ///
    /// This is conceptually the substitution map produced by unification.
    substitutions: VecMap<TypeVariable, InferType>,
}
impl TypeEnvironment {
    /// Allocates a fresh type variable and returns it as an `InferType::Var`.
    ///
    /// Each call returns a distinct variable that can later be unified
    /// with other types.
    pub(crate) fn alloc_variable(&mut self, span: Span) -> InferType {
        let variable = TypeVariable(self.next_var);
        self.next_var += 1;
        InferType::Variable(variable, span)
    }

    /// Resolves an `InferType` to its most concrete representative.
    ///
    /// If the given type is a variable and that variable has a substitution,
    /// this follows the substitution chain recursively until:
    /// - A concrete `Known` type is reached, or
    /// - An unbound `Var` is found.
    pub(crate) fn resolve(&self, ty: &InferType) -> InferType {
        match ty {
            InferType::Variable(variable, _) => {
                if let Some(infer_type) = self.substitutions.get(*variable) {
                    self.resolve(infer_type)
                } else {
                    ty.clone()
                }
            }
            _ => ty.clone(),
        }
    }

    pub(crate) fn insert_substitution(
        &mut self,
        id: TypeVariable,
        ty: InferType,
    ) -> Option<InferType> {
        self.substitutions.insert(id, ty)
    }
}
