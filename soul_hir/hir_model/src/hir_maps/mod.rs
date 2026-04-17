use soul_utils::{
    ids::{FunctionId, IdAlloc},
    soul_import_path::SoulImportPath,
    span::{ItemMetaData, ModuleId, Span},
    vec_map::VecMap,
};

use crate::{
    hir_type::Field, Block, BlockId, Expression, ExpressionId, FieldId, Function, LocalId,
    LocalInfo, Module, Place, PlaceId, StatementId,
};

mod type_map;
pub use type_map::*;

/// Auxiliary semantic metadata attached to HIR statements.
///
/// This data is not required for code generation but is useful for
/// analysis passes such as borrow checking, drop elaboration,
/// or control-flow diagnostics.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MetaDataMap {
    /// Metadata associated with statements.
    pub statements: VecMap<StatementId, ItemMetaData>,
}

/// Maps HIR node IDs to their original source code spans.
///
/// Spans originate from the AST and are forwarded through lowering passes.
/// They are used exclusively for diagnostics and are not part of the IR
/// structure itself.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpanMap {
    /// Source spans for expressions.
    pub expressions: VecMap<ExpressionId, Span>,

    /// Source spans for statements.
    pub statements: VecMap<StatementId, Span>,

    /// Source spans for blocks.
    pub blocks: VecMap<BlockId, Span>,

    pub locals: VecMap<LocalId, Span>,

    pub functions: VecMap<FunctionId, Span>,
}
impl Default for SpanMap {
    fn default() -> Self {
        Self {
            blocks: Default::default(),
            locals: Default::default(),
            functions: Default::default(),
            statements: Default::default(),
            expressions: VecMap::from_slice(&[(ExpressionId::error(), Span::error())]),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeMaps {
    pub places: VecMap<PlaceId, Place>,
    pub fields: VecMap<FieldId, Field>,
    pub blocks: VecMap<BlockId, Block>,
    pub modules: VecMap<ModuleId, Module>,
    pub locals: VecMap<LocalId, LocalInfo>,
    pub functions: VecMap<FunctionId, Function>,
    pub expressions: VecMap<ExpressionId, Expression>,
}
impl NodeMaps {
    pub fn new(init_globals: Function) -> Self {
        Self {
            places: VecMap::const_default(),
            fields: VecMap::const_default(),
            blocks: VecMap::const_default(),
            locals: VecMap::const_default(),
            modules: VecMap::const_default(),
            functions: VecMap::from_slice(&[(init_globals.id, init_globals)]),
            expressions: VecMap::from_slice(&[(
                ExpressionId::error(),
                Expression::error(ExpressionId::error()),
            )]),
        }
    }
}
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InfoMaps {
    pub imports: ImportMap,

    /// Side-table containing all types
    /// for HIR nodes.
    pub types: TypesMap,

    pub infers: InferTypesMap,

    /// Side-table containing source spans
    /// for HIR nodes.
    pub spans: SpanMap,

    /// Side-table containing auxiliary metadata
    /// for HIR nodes.
    pub meta_data: MetaDataMap,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ImportMap {
    map: VecMap<ModuleId, SoulImportPath>,
}
impl ImportMap {
    pub fn new() -> Self {
        Self { map: VecMap::new() }
    }

    pub fn insert(&mut self, id: ModuleId, path: SoulImportPath) -> Option<SoulImportPath> {
        self.map.insert(id, path)
    }

    pub fn get(&self, id: ModuleId) -> Option<&SoulImportPath> {
        self.map.get(id)
    }
}
