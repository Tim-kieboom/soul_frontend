use std::{collections::HashMap, path::Path};
use std::path::PathBuf;

use ast::{Block, Module, SoulType, Visibility};
#[cfg(debug_assertions)]
use soul_tokenizer::Token;
use soul_tokenizer::{TokenKind, TokenStream};
use soul_utils::{
    crate_store::CrateContext, error::SoulError, sementic_level::SementicFault,
    soul_names::TypeModifier, span::ModuleId, vec_set::VecSet,
};

use crate::parser::parse_utils::SEMI_COLON;

mod parse_expression;
mod parse_function;
mod parse_statement;
mod parse_type;
mod parse_utils;

/// struct used to easily see debug info about current state of Parser can be ignored outside of debug
#[cfg(debug_assertions)]
#[derive(Debug, Clone)]
pub(crate) struct DebugViewer {
    current_index: usize,
    current: Token,
}

/// Recursive descent parser that builds AST from token stream.
///
/// Manages token consumption, error recovery, scope tracking, and debug
/// information (debug builds only). Supports position save/restore for
/// backtracking during parsing.
#[derive(Debug)]
pub(crate) struct Parser<'a, 'f> {
    #[cfg(debug_assertions)]
    debug: DebugViewer,

    tokens: TokenStream<'a>,
    current_this: Option<SoulType>,
    context: &'f mut CrateContext,
    source_path: PathBuf,
}
impl<'a, 'f> Parser<'a, 'f> {
    #[cfg(not(debug_assertions))]
    fn new(
        tokens: TokenStream<'a>,
        faults: &'f mut FaultCollector,
        source_folder: PathBuf,
    ) -> Self {
        Self {
            tokens,
            faults,
            source_folder,
            path_stack: vec![],
            current_this: None,
        }
    }

    #[cfg(debug_assertions)]
    fn new(tokens: TokenStream<'a>, faults: &'f mut CrateContext, path: PathBuf) -> Self {
        use soul_tokenizer::TokenKind;
        use soul_utils::span::Span;

        let debug = DebugViewer {
            current: Token::new(TokenKind::EndLine, Span::error()),
            current_index: 0,
        };

        Self {
            debug,
            tokens,
            context: faults,
            source_path: path,
            current_this: None,
        }
    }

    pub fn parse(
        tokens: TokenStream<'a>,
        id: ModuleId,
        name: String,
        parent: Option<ModuleId>,
        context: &'f mut CrateContext,
        source_folder: PathBuf,
    ) -> Module {
        let is_capital = name.chars().next().map_or(false, char::is_uppercase);
        let visibility = if is_capital {
            Visibility::Public
        } else {
            Visibility::Private
        };

        let mut this = Self::new(tokens, context, source_folder);
        if let Err(err) = this.tokens.initialize() {
            this.log_error(err);
            return Module {
                id,
                name,
                parent,
                visibility,
                modules: VecSet::new(),
                header: HashMap::default(),
                global: Block {
                    node_id: None,
                    scope_id: None,
                    statements: vec![],
                    span: this.token().span,
                    modifier: TypeModifier::Mut,
                },
            };
        }

        #[cfg(debug_assertions)]
        {
            this.debug.current = this.token().clone();
            this.debug.current_index = 0;
        }

        let statements = this.parse_global_statments();
        Module {
            id,
            name,
            parent,
            visibility,
            modules: VecSet::new(),
            header: HashMap::default(),
            global: Block {
                statements,
                node_id: None,
                scope_id: None,
                span: this.token().span,
                modifier: TypeModifier::Mut,
            },
        }
    }

    pub fn current_path(&self) -> &Path {
        self.source_path.parent().expect("should have parent")
    }

    pub(super) fn log_error(&mut self, err: SoulError) {
        self.context.faults.push(SementicFault::error(err));
    }

    /// checked if node is end of line and ends with a semicolon
    fn ends_semicolon(&mut self) -> bool {
        self.current_is(&SEMI_COLON) && self.peek().kind == TokenKind::EndLine
    }
}
