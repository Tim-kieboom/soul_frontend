use ast_model::{AbstractSyntaxTree, ast::Block};
#[cfg(debug_assertions)]
use soul_tokenizer::Token;
use soul_tokenizer::{TokenStream};
use soul_utils::{sementic_level::SementicFault, soul_names::TypeModifier};

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
#[derive(Debug, Clone)]
pub(crate) struct Parser<'a> {
    #[cfg(debug_assertions)]
    debug: DebugViewer,

    tokens: TokenStream<'a>,
    faults: Vec<SementicFault>,
}
impl<'a> Parser<'a> {
    #[cfg(not(debug_assertions))]
    fn new(tokens: TokenStream<'a>) -> Self {
        Self {
            tokens,
            faults: vec![],
        }
    }

    #[cfg(debug_assertions)]
    fn new(tokens: TokenStream<'a>) -> Self {
        use soul_utils::span::Span;
        use soul_tokenizer::TokenKind;

        let debug = DebugViewer {
            current: Token::new(TokenKind::EndLine, Span::default_const()),
            current_index: 0,
        };

        Self {
            debug,
            tokens,
            faults: vec![],
        }
    }

    pub fn parse(tokens: TokenStream<'a>) -> (AbstractSyntaxTree, Vec<SementicFault>) {
        let mut this = Self::new(tokens);
        if let Err(err) = this.tokens.initialize() {
            this.add_error(err);
            return (
                AbstractSyntaxTree {
                    root: Block {
                        modifier: TypeModifier::Mut,
                        statements: vec![],
                        scope_id: None,
                        node_id: None,
                        span: this.token().span,
                    },
                },
                this.faults,
            );
        }

        #[cfg(debug_assertions)]
        {
            this.debug.current = this.token().clone();
            this.debug.current_index = 0;
        }

        let statements = this.parse_global_statments();
        (
            AbstractSyntaxTree {
                root: Block {
                    statements,
                    scope_id: None,
                    modifier: TypeModifier::Mut,
                    node_id: None,
                    span: this.token().span,
                },
            },
            this.faults,
        )
    }
}
