use ast_model::{
    expression::{Expression, Lambda},
    statements::VarPattern,
};
use soul_tokenizer::model::TokenKind;
use soul_utils::{
    TypeModifier, soul_names::Symbol, span::Span,
};

use crate::{
    parser::Parser,
    utils::LAMBDA_ARROW,
};

impl<'a, 'f> Parser<'a, 'f> {
    /// Try to parse a lambda expression: `params => body`.
    /// Returns `Ok(expr)` if successful, or `Err(())` on failure (caller should backtrack).
    pub(super) fn try_parse_lambda(&mut self, start_span: Span) -> Result<Expression, ()> {
        let saved = self.tokens.current_position();

        let pattern = self.parse_var_pattern(TypeModifier::Const).map_err(|_| ())?;

        if self.current_is(&LAMBDA_ARROW) {
            self.bump();
            let body = self.parse_expression_id(&LAMBDA_BODY_END).map_err(|_| ())?;
            let params = match pattern {
                VarPattern::Tuple(tuple) => tuple.elements,
                other => vec![other],
            };
            Ok(Expression::from_lambda(
                Lambda {
                    id: self.alloc_node(),
                    parameters: params,
                    body,
                },
                self.span_combine(start_span),
            ))
        } else {
            self.goto(saved);
            Err(())
        }
    }
}

const LAMBDA_BODY_END: &[TokenKind] = &[
    TokenKind::EndFile,
    TokenKind::EndLine,
    TokenKind::Symbol(Symbol::SemiColon),
    TokenKind::Symbol(Symbol::RoundClose),
    TokenKind::Symbol(Symbol::CurlyClose),
    TokenKind::Symbol(Symbol::SquareClose),
    TokenKind::Symbol(Symbol::Comma),
    TokenKind::Symbol(Symbol::Colon),
];

