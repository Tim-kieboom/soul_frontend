use ast::{AbstractSyntaxTree, Block};
#[cfg(debug_assertions)]
use soul_tokenizer::Token;
use soul_tokenizer::{TokenKind, TokenStream};
use soul_utils::{sementic_level::SementicFault, soul_names::TypeModifier};

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
    faults: &'f mut Vec<SementicFault>,
}
impl<'a, 'f> Parser<'a, 'f> {
    #[cfg(not(debug_assertions))]
    fn new(tokens: TokenStream<'a>) -> Self {
        Self {
            tokens,
            faults: vec![],
        }
    }

    #[cfg(debug_assertions)]
    fn new(tokens: TokenStream<'a>, faults: &'f mut Vec<SementicFault>) -> Self {
        use soul_tokenizer::TokenKind;
        use soul_utils::span::Span;

        let debug = DebugViewer {
            current: Token::new(TokenKind::EndLine, Span::default_const()),
            current_index: 0,
        };

        Self {
            debug,
            tokens,
            faults,
        }
    }

    pub fn parse(
        tokens: TokenStream<'a>,
        faults: &'f mut Vec<SementicFault>,
    ) -> AbstractSyntaxTree {
        let mut this = Self::new(tokens, faults);
        if let Err(err) = this.tokens.initialize() {
            this.log_error(err);
            return AbstractSyntaxTree {
                root: Block {
                    modifier: TypeModifier::Mut,
                    statements: vec![],
                    scope_id: None,
                    node_id: None,
                    span: this.token().span,
                },
            };
        }

        #[cfg(debug_assertions)]
        {
            this.debug.current = this.token().clone();
            this.debug.current_index = 0;
        }

        let statements = this.parse_global_statments();
        AbstractSyntaxTree {
            root: Block {
                statements,
                scope_id: None,
                modifier: TypeModifier::Mut,
                node_id: None,
                span: this.token().span,
            },
        }
    }

    /// checked if node is end of line and ends with a semicolon
    fn ends_semicolon(&mut self) -> bool {
        self.current_is(&SEMI_COLON) && self.peek().kind == TokenKind::EndLine
    }
}
