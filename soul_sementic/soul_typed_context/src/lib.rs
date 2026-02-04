//! Typed HIR Analysis
//!
//! This crate provides type inference and checking for HIR (High-level Intermediate Representation)
//! trees. It implements a classic Hindley-Milner style algorithm with unification-based type inference.
//!
//! ## Core Concepts
//!
//! - **`InferType`**: Types during inference are either concrete `HirType`s or unification variables
//! - **`TypeEnvironment`**: Global state managing type variable allocation and substitutions  
//! - **`TypedContext`**: Per-tree context that tracks expression types, local bindings, and errors
//!
//! ## Usage
//!
//! ```
//! use soul_typed_hir::get_typed_hir;
//!
//! let hir_tree = /*soul_hir::lower_ast(ast)*/;
//! let errors = get_typed_hir(&hir_tree);
//! ```
//!
//! The main entry point `get_typed_hir()` walks an HIR tree and returns any semantic/type errors.
//! Inferred types are stored internally and can be extended to expose a fully-typed HIR.

use hir_model::{ExpressionId, HirTree, HirType};
use parser_models::scope::NodeId;
use soul_utils::{
    error::{SoulError, SoulErrorKind},
    sementic_level::SementicFault,
    vec_map::{VecMap, VecSet},
};

use crate::model::{InferType, TypeEnvironment};

mod handle_type;
mod infer_expression;
mod infer_item;
mod infer_statement;
mod model;
mod utils;

pub struct TypedHirResponse {
    pub typed_context: VecMap<NodeId, HirType>,
    pub auto_copys: VecSet<ExpressionId>,
    pub faults: Vec<SementicFault>,
}

/// Perform type inference on the given HIR tree.
///
/// This walks the HIR, infers types for expressions and locals,
/// records any semantic/type errors as `SementicFault`s,
/// and returns the collected faults.
///
/// The actual inferred types are stored internally in `TypedContext`
/// and can be extended later to be exposed if needed.
pub fn get_typed_context(tree: &HirTree) -> TypedHirResponse {
    TypedContext::infer_types(tree).convert_to_response()
}

/// Type-checking context for a single HIR tree.
///
/// This holds:
/// - A reference to the HIR (`tree`).
/// - Maps from expressions and locals to their inferred types.
/// - The global type inference environment.
/// - Any faults found during inference.
pub struct TypedContext<'a> {
    tree: &'a HirTree,

    auto_copys: VecSet<ExpressionId>,

    /// Inferred types for each expression node in the HIR.
    expression_types: VecMap<ExpressionId, InferType>,

    /// Inferred types for local bindings (variables, parameters, etc.).
    locals: VecMap<NodeId, InferType>,
    environment: TypeEnvironment,

    faults: Vec<SementicFault>,
    current_return_type: Option<InferType>,
    current_return_count: usize,
}
impl<'a> TypedContext<'a> {
    fn convert_to_response(mut self) -> TypedHirResponse {
        let mut faults = std::mem::take(&mut self.faults);
        let mut types = VecMap::with_capacity(self.locals.cap());
        for (node_id, infer_type) in self
            .locals
            .into_entries()
            .chain(self.expression_types.into_entries())
        {
            match infer_type {
                InferType::Known(hir_type) => _ = types.insert(node_id, hir_type),
                InferType::Variable(_, span) => {
                    faults.push(SementicFault::error(SoulError::new(
                        "type was not resolved",
                        SoulErrorKind::InternalError(file!().to_string(), line!()),
                        Some(span),
                    )));
                }
            };
        }

        TypedHirResponse {
            faults,
            typed_context: types,
            auto_copys: self.auto_copys,
        }
    }

    fn infer_types(tree: &'a HirTree) -> Self {
        let mut this = Self::new(tree);

        for item in tree.root.items.values() {
            this.infer_item(item);
        }

        this
    }

    fn new(tree: &'a HirTree) -> Self {
        Self {
            tree,
            faults: vec![],
            locals: VecMap::new(),
            current_return_type: None,
            expression_types: VecMap::new(),
            environment: TypeEnvironment::default(),
            current_return_count: 0,
            auto_copys: VecSet::new(),
        }
    }
}
