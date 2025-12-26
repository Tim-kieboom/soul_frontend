use crate::steps::sementic_analyser::{SementicInfo, SementicPass, sementic_fault::SementicFault};
use models::abstract_syntax_tree::AbstractSyntaxTree;
use models::{
    abstract_syntax_tree::{soul_type::NamedTupleType, statment::Ident},
    error::{SoulError, SoulErrorKind, Span},
    sementic_models::scope::{
        NodeId, NodeIdGenerator, Scope, ScopeTypeEntry, ScopeTypeKind, ScopeValueEntry,
        ScopeValueEntryKind, ScopeValueKind,
    },
};

pub struct NameResolver<'a> {
    pub ids: NodeIdGenerator,
    pub info: &'a mut SementicInfo,

    pub current_function: Option<NodeId>,
}

impl<'a> SementicPass<'a> for NameResolver<'a> {
    fn new(info: &'a mut SementicInfo) -> Self {
        Self {
            info,
            current_function: None,
            ids: NodeIdGenerator::new(),
        }
    }

    fn run(&mut self, ast: &mut AbstractSyntaxTree) {
        self.resolve_block(&mut ast.root);
    }
}

impl<'a> NameResolver<'a> {


    

    pub(super) fn declare_parameters(&mut self, parameters: &mut NamedTupleType) {
        for (name, _ty, node_id) in &mut parameters.types {
            let id = self.new_id();
            *node_id = Some(id);

            self.insert_value(
                name.clone(),
                ScopeValueEntry {
                    node_id: id,
                    kind: ScopeValueEntryKind::Variable,
                },
            );
        }
    }

    pub(super) fn declare_value(&mut self, mut value: ScopeValueKind) -> NodeId {
        let id = self.new_id();
        *value.get_id_mut() = Some(id);

        self.insert_value(
            value.get_name().clone(),
            ScopeValueEntry {
                node_id: id,
                kind: value.to_entry_kind(),
            },
        );

        id
    }

    pub(super) fn declare_type(&mut self, mut ty: ScopeTypeKind, span: Span) {
        let id = self.new_id();
        *ty.get_id_mut() = Some(id);

        let name = ty.get_name().clone();
        let entry = ScopeTypeEntry {
            node_id: id,
            span,
            kind: ty.to_entry_kind(),
        };

        let same_type = match self.insert_types(name, entry) {
            Some(val) => val,
            None => return,
        };

        self.log_error(SoulError::new(
            format!(
                "more then one of typename '{}' exist in this scope",
                ty.get_name().as_str(),
            ),
            SoulErrorKind::ScopeOverride(same_type.span),
            Some(span),
        ));
    }

    pub(super) fn log_error(&mut self, err: SoulError) {
        self.info.faults.push(SementicFault::error(err));
    }

    fn insert_value(&mut self, name: Ident, entry: ScopeValueEntry) {
        self.current_scope_mut()
            .values
            .entry(name.node)
            .or_default()
            .push(entry);
    }

    fn insert_types(&mut self, name: Ident, entry: ScopeTypeEntry) -> Option<ScopeTypeEntry> {
        self.current_scope_mut().types.insert(name.node, entry)
    }

    fn new_id(&mut self) -> NodeId {
        self.ids.new_id()
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        .expect("resolver has no scope")
    }
}
