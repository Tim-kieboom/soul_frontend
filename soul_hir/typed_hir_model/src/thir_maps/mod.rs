use soul_utils::{bimap::BiMap, ids::{FunctionId, IdAlloc, IdGenerator}, soul_import_path::SoulImportPath, span::{ItemMetaData, Span}, vec_map::VecMap};

use hir::{BlockId, ExpressionId, FieldId, LocalId, ModuleId, PlaceId, StatementId, TypesMap};

mod type_map;

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
            expressions: VecMap::from_slice(&[(ExpressionId::error(), Span::default_const())]),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeMaps {
    pub places: VecMap<PlaceId, crate::Place>,
    pub fields: VecMap<FieldId, crate::Field>,
    pub blocks: VecMap<BlockId, crate::Block>,
    pub locals: VecMap<LocalId, crate::LocalInfo>,
    pub functions: VecMap<FunctionId, crate::Function>,
    pub expressions: VecMap<ExpressionId, crate::Expression>,
}
impl NodeMaps {
    pub fn new(init_globals: crate::Function) -> Self {
        Self { 
            places: VecMap::const_default(),
            fields: VecMap::const_default(),
            blocks: VecMap::const_default(),
            locals: VecMap::const_default(),
            functions: VecMap::from_slice(&[(init_globals.id, init_globals)]),
            expressions: VecMap::from_slice(&[(ExpressionId::error(), crate::Expression::error(ExpressionId::error()))]),
        }
    }
}
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InfoMaps {
    pub imports: ImportMap,

    /// Side-table containing all types
    /// for HIR nodes.
    pub types: TypesMap,

    /// Side-table containing source spans
    /// for HIR nodes.
    pub spans: SpanMap,

    /// Side-table containing auxiliary metadata
    /// for HIR nodes.
    pub meta_data: MetaDataMap,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ImportMap {
    map: BiMap<ModuleId, SoulImportPath>,
}
impl ImportMap {
    pub fn new() -> Self {
        Self { map: BiMap::new() }
    }

    pub fn insert(&mut self, alloc: &mut IdGenerator<ModuleId>, path: SoulImportPath) -> ModuleId {
        self.map.insert(alloc, path)
    }

    pub fn get_module(&self, id: ModuleId) -> Option<&SoulImportPath> {
        self.map.get_value(id)
    }

    pub fn get_id(&self, value: &SoulImportPath) -> Option<ModuleId> {
        self.map.get_key(value)
    }
}