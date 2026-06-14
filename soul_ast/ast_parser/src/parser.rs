use ast_model::{AstStore, Module, block::Block};
use soul_tokenizer::TokenStream;
#[cfg(debug_assertions)]
use soul_tokenizer::model::Token;
use soul_utils::{CrateContext, TypeModifier, collections::vec_set::VecSet};
use std::{collections::HashMap, path::PathBuf};

use crate::ParseInfo;

/// struct used to easily see debug info about current state of Parser can be ignored outside of debug
#[cfg(debug_assertions)]
#[derive(Debug, Clone)]
pub(crate) struct DebugViewer {
    pub(crate) current_index: usize,
    pub(crate) current: Token,
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

    pub(crate) tokens: TokenStream<'a>,
    pub(crate) store: &'f mut AstStore,
    pub(crate) context: &'f mut CrateContext,
    pub(crate) source_path: PathBuf,
}
impl<'a, 'f> Parser<'a, 'f> {
    pub fn parse(tokens: TokenStream<'a>, info: ParseInfo<'f>) -> Module {
        let mut this = Self::new(tokens, info.store, info.context, info.source_folder);

        #[cfg(debug_assertions)]
        {
            this.debug.current = this.token().clone();
            this.debug.current_index = this.tokens.index();
        }

        let statements = this.parse_global_statements();
        let global = this.store.insert_block(Block {
            statements,
            node_id: None,
            scope_id: None,
            span: this.token().span,
            modifier: TypeModifier::Mut,
        });

        Module {
            global,
            id: info.id,
            name: info.name,
            parent: info.parent,
            modules: VecSet::new(),
            header: HashMap::default(),
        }
    }

    #[cfg(not(debug_assertions))]
    fn new(
        tokens: TokenStream<'a>,
        store: &'f mut AstStore,
        context: &'f mut CrateContext,
        source_path: PathBuf,
    ) -> Self {
        Self {
            tokens,
            store,
            context,
            source_path,
            current_this: None,
        }
    }

    #[cfg(debug_assertions)]
    fn new(
        tokens: TokenStream<'a>,
        store: &'f mut AstStore,
        context: &'f mut CrateContext,
        source_path: PathBuf,
    ) -> Self {
        use soul_tokenizer::model::TokenKind;
        use soul_utils::span::Span;

        let debug = DebugViewer {
            current: Token::new(TokenKind::EndLine, Span::error()),
            current_index: 0,
        };

        Self {
            debug,
            tokens,
            store,
            context,
            source_path,
        }
    }
}
