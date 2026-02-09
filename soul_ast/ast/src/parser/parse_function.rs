use ast_model::ast::{
    Expression, Function, FunctionCall, FunctionSignature, SoulType, Statement,
};
use soul_utils::{
    Ident,
    error::{SoulError, SoulResult},
    soul_names::TypeModifier,
    span::{Span, Spanned},
    try_result::{ResultTryErr, ToResult, TryErr, TryError, TryNotValue, TryOk, TryResult},
};

use crate::parser::{
    Parser,
    parse_utils::{ARROW_LEFT, COLON, COMMA, ROUND_CLOSE, ROUND_OPEN},
};

type FuncResult<T> = TryResult<T, (Ident, Box<SoulError>)>;
impl<'a> Parser<'a> {
    pub(crate) fn parse_any_function(&mut self) -> SoulResult<Statement> {
        fn default_methode_type(span: Span) -> SoulType {
            SoulType::none(span).with_modifier(Some(TypeModifier::Mut))
        }

        let ident = self.try_bump_consume_ident()?;

        let span = self.token().span;
        match self.try_parse_function_declaration(span, default_methode_type(span), ident) {
            Ok(val) => Ok(Statement::from_function(val)),
            Err(TryError::IsErr(err)) => Err(err),
            Err(TryError::IsNotValue((ident, _err))) => self
                .try_parse_function_call(span, None, &ident)
                .merge_to_result()
                .map(Statement::from_function_call),
        }
    }

    pub(crate) fn try_parse_function_declaration(
        &mut self,
        start_span: Span,
        methode_type: SoulType,
        name: Ident,
    ) -> FuncResult<Spanned<Function>> {
        
        let begin = self.current_position();
        let modifier = methode_type
            .modifier
            .unwrap_or(TypeModifier::Mut);

        let signature = self.try_parse_function_signature(start_span, methode_type, name)?;

        let block = match self.parse_block(modifier) {
            Ok(val) => val,
            Err(err) => if signature.node.parameters.is_empty() {
                self.go_to(begin);
                return TryNotValue((signature.node.name, Box::new(err)))
            } else {
                return TryErr(err)
            },
        };

        let span = signature.span;
        Ok(Spanned::new(
            Function {
                block,
                signature,
                node_id: None,
            },
            self.span_combine(span),
        ))
    }

    pub(crate) fn try_parse_function_call(
        &mut self,
        start_span: Span,
        callee: Option<&Expression>,
        name: &Ident,
    ) -> TryResult<Spanned<FunctionCall>, SoulError> {
        if !self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return TryNotValue(self.get_expect_any_error(&[ROUND_OPEN, ARROW_LEFT]));
        }

        let arguments = self.parse_arguments().try_err()?;
        TryOk(Spanned::new(
            FunctionCall {
                name: name.clone(),
                callee: callee.map(|expr| Box::new(expr.clone())),
                arguments,
                id: None,
                resolved: None,
            },
            self.span_combine(start_span),
        ))
    }

    pub(crate) fn try_parse_function_signature(
        &mut self,
        start_span: Span,
        methode_type: SoulType,
        name: Ident,
    ) -> FuncResult<Spanned<FunctionSignature>> {
        let begin_position = self.current_position();
        let result = self.inner_parse_function_signature(start_span, methode_type, name);

        if result.is_err() {
            self.go_to(begin_position);
        }

        result
    }

    fn parse_arguments(&mut self) -> SoulResult<Vec<Expression>> {
        self.expect(&ROUND_OPEN)?;
        if self.current_is(&ROUND_CLOSE) {
            self.bump();
            return Ok(vec![]);
        }

        let mut values = vec![];
        loop {
            let value = self.parse_expression(&[COMMA, ROUND_CLOSE])?;
            values.push(value);
            if !self.current_is(&COMMA) {
                break;
            }

            self.bump();
        }

        self.expect(&ROUND_CLOSE)?;
        Ok(values)
    }

    fn inner_parse_function_signature(
        &mut self,
        start_span: Span,
        methode_type: SoulType,
        name: Ident,
    ) -> FuncResult<Spanned<FunctionSignature>> {
        if !self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return TryNotValue((
                name,
                Box::new(self.get_expect_any_error(&[ROUND_OPEN, ARROW_LEFT])),
            ));
        }

        if !self.current_is(&ROUND_OPEN) {
            return TryErr(self.get_expect_error(&ROUND_OPEN));
        }

        let (parameters, function_kind) = match self.try_parse_parameters() {
            Ok(val) => val,
            Err(TryError::IsErr(err)) => return TryErr(err),
            Err(TryError::IsNotValue(err)) => return TryNotValue((name, Box::new(err))),
        };

        let return_type = match self.current_is(&COLON) {
            true => {
                self.bump();
                match self.try_parse_type() {
                    Ok(val) => val,
                    Err(TryError::IsErr(err)) => return TryErr(err),
                    Err(TryError::IsNotValue(err)) => return TryNotValue((name, Box::new(err))),
                }
            }
            false => SoulType::none(self.token().span),
        };

        let signature = FunctionSignature {
            name,
            parameters,
            return_type,
            methode_type,
            function_kind,
        };

        TryOk(Spanned::new(signature, self.span_combine(start_span)))
    }
}
