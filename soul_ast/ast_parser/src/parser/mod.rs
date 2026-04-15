use ast::{Block, Module, SoulType, Visibility};
#[cfg(debug_assertions)]
use soul_tokenizer::Token;
use soul_tokenizer::{TokenKind, TokenStream};
use soul_utils::{sementic_level::CompilerContext, soul_names::TypeModifier, span::ModuleId};

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
    context: &'f mut CompilerContext,
}
impl<'a, 'f> Parser<'a, 'f> {
    #[cfg(not(debug_assertions))]
    fn new(tokens: TokenStream<'a>, context: &'f mut CompilerContext) -> Self {
        Self {
            tokens,
            context,
            current_this: None,
        }
    }

    #[cfg(debug_assertions)]
    fn new(tokens: TokenStream<'a>, context: &'f mut CompilerContext) -> Self {
        use soul_tokenizer::TokenKind;
        use soul_utils::span::Span;

        let debug = DebugViewer {
            current: Token::new(TokenKind::EndLine, Span::error()),
            current_index: 0,
        };

        Self {
            debug,
            tokens,
            context,
            current_this: None,
        }
    }

    pub fn parse(
        tokens: TokenStream<'a>, 
        id: ModuleId,
        name: String,
        context: &'f mut CompilerContext,
    ) -> Module {
        let is_capital = name.chars().next().map_or(false, char::is_uppercase);
        let visibility = if is_capital {
            Visibility::Public
        } else {
            Visibility::Private
        };
        
        let mut this = Self::new(tokens, context);
        if let Err(err) = this.tokens.initialize() {
            this.log_error(err);
            return Module {
                id,
                name,
                visibility,
                modules: vec![],
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
            visibility,
            modules: vec![],
            global: Block {
                statements,
                node_id: None,
                scope_id: None,
                span: this.token().span,
                modifier: TypeModifier::Mut,
            },
        }
    }

    /// checked if node is end of line and ends with a semicolon
    fn ends_semicolon(&mut self) -> bool {
        self.current_is(&SEMI_COLON) && self.peek().kind == TokenKind::EndLine
    }
}
