mod expression;
mod function;
mod hir_type;
mod ids;
mod meta_data_maps;
mod place;
mod statement;
use ast::FunctionKind;
pub use expression::*;
pub use function::*;
pub use hir_type::*;
pub use ids::*;
pub use meta_data_maps::*;
pub use place::*;
use soul_utils::{
    Ident,
    ids::{FunctionId, IdAlloc},
    span::Span,
    vec_map::VecMap,
};
pub use statement::*;

/// High-level Intermediate Representation (HIR) tree.
///
/// The HIR contains the fully resolved and typed structure of a module.
/// It is free of syntax-only information such as tokens or spans.
///
/// Source locations and auxiliary semantic information are stored separately
/// in `MetaDataMap` and are indexed by IR node IDs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirTree {
    /// Root module of the HIR.
    pub root: Module,

    /// Side-table containing all types
    /// for HIR nodes.
    pub types: TypesMap,

    /// Side-table containing source spans
    /// for HIR nodes.
    pub spans: SpanMap,

    /// Side-table containing auxiliary metadata
    /// for HIR nodes.
    pub meta_data: MetaDataMap,

    /// id for initialize runtime globals function created in mir
    pub init_global_function: FunctionId,
    pub main_function: FunctionId,

    pub internal_fields: Option<InternalFields>,

    pub imports: ImportMap,
    pub fields: VecMap<FieldId, Field>,
    pub blocks: VecMap<BlockId, Block>,
    pub locals: VecMap<LocalId, LocalInfo>,
    pub functions: VecMap<FunctionId, Function>,
    pub expressions: VecMap<ExpressionId, Expression>,
}

impl HirTree {
    pub fn new(root_id: ModuleId, main_function: FunctionId, init_global_function_id: FunctionId) -> Self {
        let init_global_function = Function {
            id: init_global_function_id,
            name: Ident::new("_start".to_string(), Span::default_const()),
            parameters: vec![],
            kind: FunctionKind::Static,
            return_type: TypeId::error(),
            body: FunctionBody::Internal(BlockId::error()),
        };

        Self {
            internal_fields: None,
            main_function,
            init_global_function: init_global_function_id,
            types: TypesMap::new(),
            spans: SpanMap::default(),
            blocks: VecMap::default(),
            locals: VecMap::default(),
            fields: VecMap::default(),
            imports: ImportMap::new(),
            expressions: VecMap::from_slice(&[(ExpressionId::error(), Expression::error(ExpressionId::error()))]),
            meta_data: MetaDataMap::default(),
            functions: VecMap::from_vec(vec![(init_global_function_id, init_global_function)]),
            root: Module {
                id: root_id,
                imports: vec![],
                globals: vec![],
            },
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalInfo {
    pub ty: TypeId,
    pub kind: LocalKind,
}
impl LocalInfo {
    pub fn is_temp(&self) -> bool {
        self.kind == LocalKind::Temp
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LocalKind {
    Variable,
    Parameter,
    Temp,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InternalFields {
    pub array_ptr: FieldId,
    pub array_len: FieldId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub id: FieldId,
    /// The name of the field (for readability, debugging, codegen).
    pub name: String,
    /// The type of the field in HIR.
    pub type_id: TypeId,
    /// stores alignmentInfo
    pub align: Alignment,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Alignment {
    /// Byte offset of the start of the field within its parent struct.
    pub offset: usize,
    /// Size of the field in bytes (computed from `type_id`).
    pub size: usize,
    /// Alignment requirement of this field in bytes.
    pub align: usize,
    /// Whether this field is padded or part of a packed struct.
    pub is_packed: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Module {
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
pub enum Global {
    Function(FunctionId, StatementId),
    Variable(Variable, StatementId),

    /// (allows mutable) only allowed to be used by hir lowerer not user
    InternalVariable(Variable, StatementId),
    /// only allowed to be used by hir lowerer not user
    InternalAssign(Assign, StatementId),
}
impl Global {
    pub fn get_id(&self) -> StatementId {
        match self {
            Global::Function(_, statement_id) => *statement_id,
            Global::Variable(_, statement_id) => *statement_id,
            Global::InternalAssign(_, statement_id) => *statement_id,
            Global::InternalVariable(_, statement_id) => *statement_id,
        }
    }

    pub fn get_span(&self, spans: &SpanMap) -> Span {
        let span = match self {
            Global::Variable(variable, _) => spans.locals.get(variable.local),
            Global::Function(function_id, _) => spans.functions.get(*function_id),
            Global::InternalAssign(assign, _) => spans.expressions.get(assign.value),
            Global::InternalVariable(variable, _) => spans.locals.get(variable.local),
        };

        match span {
            Some(val) => *val,
            None => {
                #[cfg(debug_assertions)]
                panic!("span not found of {:?}", self);
                #[cfg(not(debug_assertions))]
                Span::default_const()
            }
        }
    }

    pub const fn should_be_inmutable(&self) -> bool {
        match self {
            Global::Function(_, _) | Global::InternalVariable(_, _) => false,

            Global::Variable(_, _) | Global::InternalAssign(_, _) => true,
        }
    }
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
    pub statements: Vec<Statement>,
    pub terminator: Option<ExpressionId>,
}
impl Block {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            imports: vec![],
            terminator: None,
            statements: vec![],
        }
    }
}
