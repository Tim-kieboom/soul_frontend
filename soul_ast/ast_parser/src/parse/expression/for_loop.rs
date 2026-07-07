use ast_model::{
    expression::{Binding, ExpressionId, For, ForCondition},
    statements::VarPattern,
};
use soul_utils::{
    TypeModifier,
    collections::try_result::{ResultTryErr, ResultTryNotValue, TryError, TryOk, TryResult},
    error::SoulResult,
    fault::Fault,
    soul_names::Symbol,
    span::Spanned,
};

use crate::{
    parser::Parser,
    utils::{COMMA, CURLY_OPEN, FOR, IN},
};

impl<'a, 'f> Parser<'a, 'f> {
    pub fn parse_for_loop(&mut self) -> SoulResult<Spanned<For>> {
        let start_span = self.token().span;
        self.expect(&FOR)?;

        let condition = match &self.token().kind {
            &CURLY_OPEN => ForCondition::Loop,
            _ => {
                let saved = self.tokens.current_position();

                let index = if self.peek_is(&COMMA) {
                    let ident = self.try_bump_consume_ident()?;
                    self.bump();
                    Some(Binding::new(self.alloc_node(), ident))
                } else {
                    None
                };

                match self.try_parse_foreach_elements() {
                    Ok((element, collection)) => ForCondition::Foreach {
                        index,
                        collection,
                        element_kind: element,
                    },
                    Err(TryError::IsNotValue(())) => {
                        if index.is_some() {
                            return Err(Fault::error(
                                format!("`{}` is invalid", Symbol::Comma.as_str()),
                                Some(self.span_combine(start_span)),
                            ));
                        }

                        self.goto(saved);
                        let value = self.parse_expression_id(&[CURLY_OPEN])?;
                        ForCondition::While(value)
                    }
                    Err(TryError::IsErr(err)) => return Err(err),
                }
            }
        };

        let block = self.parse_block(TypeModifier::Mut)?;
        Ok(Spanned::new(
            For { block, condition },
            self.span_combine(start_span),
        ))
    }

    fn try_parse_foreach_elements(&mut self) -> TryResult<(VarPattern, ExpressionId), ()> {
        let var_pattern = self
            .parse_var_pattern(TypeModifier::Const)
            .try_not_value()?;
        self.expect(&IN).try_not_value()?;
        let collection = self.parse_expression_id(&[CURLY_OPEN]).try_err()?;
        TryOk((var_pattern, collection))
    }
}
