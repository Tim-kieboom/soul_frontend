use hir::{BlockId, ExpressionId, FieldId, LocalId, ModuleId, PlaceId, StatementId, StructId, TypeId};
use soul_utils::{ids::FunctionId, span::Span};

mod function;
mod thir_maps;
mod statement;
mod expression;
pub use function::*;
pub use statement::*;
pub use thir_maps::*; 
pub use expression::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypedHirTree {
    pub root: TypedModule,
    pub info: InfoMaps,
    pub nodes: NodeMaps,

    pub main: FunctionId,
    pub init_globals: FunctionId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypedModule {
    pub id: ModuleId,
    pub imports: Vec<Import>,
    pub globals: Vec<Global>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Import {
    pub module: ModuleId,
    pub kind: ast::ImportKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Global {
    pub id: StatementId,
    pub kind: crate::GlobalKind,
} 

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GlobalKind {
    Function(FunctionId),
    Variable(crate::Variable),

    /// (allows mutable) only allowed to be used by hir lowerer not user
    InternalVariable(crate::Variable),
    /// only allowed to be used by hir lowerer not user
    InternalAssign(crate::Assign),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Variable {
    pub ty: TypeId,
    pub is_temp: bool,
    pub local: LocalId,
    pub value: Option<ExpressionId>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub imports: Vec<Import>,
    pub statements: Vec<crate::Statement>,
    pub terminator: Option<ExpressionId>,
}

/// A memory location that can be read from or written to.
///
/// Places represent l-values in the language and are used for
/// assignments, loads, references, and indexing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Place {
    pub id: PlaceId,
    pub kind: PlaceKind,
    pub span: Span,
}
/// A memory location that can be read from or written to.
///
/// Places represent l-values in the language and are used for
/// assignments, loads, references, and indexing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PlaceKind {
    /// A local variable.
    Local(LocalId),

    /// A temperary local variable.
    Temp(LocalId),

    /// Dereference of another place.
    Deref(PlaceId),

    /// Indexed access into an aggregate.
    Index {
        base: PlaceId,
        index: ExpressionId,
    },

    /// Field access within a composite type.
    Field {
        base: PlaceId,
        field: FieldId,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalInfo {
    pub ty: TypeId,
    pub kind: LocalKind,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LocalKind {
    Variable(Option<ExpressionId>),
    Temp(ExpressionId),
    Parameter,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Struct {
    pub id: StructId,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub id: FieldId,
    /// The name of the field (for readability, debugging, codegen).
    pub name: String,
    pub ty: TypeId,
}