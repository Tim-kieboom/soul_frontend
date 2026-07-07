use ast_model::{
    AstModuleStore, AstStore, CrateBoundary, Module,
    block::{Block, BlockId},
    soul_type::SoulType,
};
use soul_tokenizer::TokenStream;
#[cfg(debug_assertions)]
use soul_tokenizer::model::Token;
use soul_utils::{
    CrateContext, TypeModifier, collections::vec_set::VecSet, ids::IdAlloc, soul_error_internal,
};
use soul_utils::{
    collections::{crate_store::CrateStore, module_store::ModuleStore},
    span::ModuleId,
};
use std::{collections::HashMap, path::PathBuf};

use crate::ParseInfo;

/// struct used to easily see debug info about current state of Parser can be ignored outside of debug
#[cfg(debug_assertions)]
#[derive(Debug, Clone)]
pub(crate) struct DebugViewer {
    pub(crate) current_index: usize,
    pub(crate) current: Token,
}

#[derive(Debug)]
pub(crate) struct Current {
    pub(crate) this_type: Option<SoulType>,
}
impl Default for Current {
    fn default() -> Self {
        Self { this_type: None }
    }
}

/// Recursive descent parser that builds AST from token stream.
///
/// Manages token consumption, error recovery, scope tracking, and debug
/// information (debug builds only). Supports position save/restore for
/// backtracking during parsing.
#[derive(Debug)]
pub(crate) struct Parser<'a, 'f> {
    #[cfg(debug_assertions)]
    pub(crate) debug: DebugViewer,

    pub(crate) id: ModuleId,
    pub(crate) current: Current,
    pub(crate) source_path: PathBuf,
    pub(crate) tokens: TokenStream<'a>,
    pub(crate) store: &'f mut AstStore,
    pub(crate) crate_source_path: PathBuf,
    pub(crate) modules: &'f mut ModuleStore,
    pub(crate) context: &'f mut CrateContext,
    pub(crate) ast_modules: &'f mut AstModuleStore,
    pub(crate) crate_boundaries: &'f mut HashMap<ModuleId, CrateBoundary>,
    pub(crate) crate_store: &'f CrateStore,
}
impl<'a, 'f> Parser<'a, 'f> {
    pub fn parse(tokens: TokenStream<'a>, name: String, info: ParseInfo<'f>) {
        let id = info.id;
        let parent = info.parent;

        let module = Module {
            id,
            name,
            parent,
            modules: VecSet::new(),
            global: BlockId::error(),
            header: HashMap::default(),
        };
        info.ast_modules.insert(id, module);

        let mut this = Self::new(tokens, info);

        #[cfg(debug_assertions)]
        {
            this.debug.current = this.token().clone();
            this.debug.current_index = this.tokens.index();
        }

        let statements = this.parse_global_statements();
        let global = this.store.insert_block(Block {
            statements,
            span: this.token().span,
            modifier: TypeModifier::Mut,
        });

        match this.ast_modules.get_mut(id) {
            Some(module) => module.global = global,
            None => this.log_fault(soul_error_internal!(format!("{id:?} not found"), None)),
        }
    }

    #[cfg(not(debug_assertions))]
    fn new(tokens: TokenStream<'a>, info: ParseInfo<'f>) -> Self {
        Self {
            tokens,
            id: info.id,
            store: info.store,
            context: info.context,
            modules: info.modules,
            ast_modules: info.ast_modules,
            crate_boundaries: info.crate_boundaries,
            source_path: info.source_folder,
            crate_source_path: info.crate_source_folder,
            crate_store: info.crate_store,
            current: Current::default(),
        }
    }

    #[cfg(debug_assertions)]
    fn new(tokens: TokenStream<'a>, info: ParseInfo<'f>) -> Self {
        use soul_tokenizer::model::TokenKind;
        use soul_utils::span::Span;

        let debug = DebugViewer {
            current: Token::new(TokenKind::EndLine, Span::error()),
            current_index: 0,
        };

        Self {
            debug,
            tokens,
            id: info.id,
            store: info.store,
            context: info.context,
            modules: info.modules,
            ast_modules: info.ast_modules,
            crate_boundaries: info.crate_boundaries,
            source_path: info.source_folder,
            crate_source_path: info.crate_source_folder,
            crate_store: info.crate_store,
            current: Current::default(),
        }
    }
}
