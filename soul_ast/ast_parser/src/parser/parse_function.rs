use ast::{
    Argument, Expression, ExternLanguage, Function, FunctionCall, FunctionSignature, Generic,
    SoulType, Statement,
};
use soul_tokenizer::TokenKind;
use soul_utils::{
    Ident,
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{KeyWord, TypeModifier},
    span::{Span, Spanned},
    try_result::{ResultTryErr, ToResult, TryErr, TryError, TryNotValue, TryOk, TryResult},
};

use crate::parser::{
    Parser,
    parse_utils::{
        ARROW_LEFT, ARROW_RIGHT, COLON, COMMA, CURLY_OPEN, ROUND_CLOSE, ROUND_OPEN, SEMI_COLON,
    },
};

type FuncResult<T> = TryResult<T, (Ident, Box<SoulError>)>;
impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_any_function(&mut self) -> SoulResult<Statement> {

        let mut ident = self.try_bump_consume_ident()?;
        let modifier = match TypeModifier::from_str(ident.as_str()) {
            Some(modifer) => {
                ident = self.try_bump_consume_ident()?;
                modifer
            }
            None => TypeModifier::Mut,
        };

        let span = self.token().span;
        match self.try_parse_function_declaration(span, self.default_methode_type(modifier, span), ident) {
            Ok(val) => Ok(Statement::from_function(val)),
            Err(TryError::IsErr(err)) => Err(err),
            Err(TryError::IsNotValue((ident, _err))) => self
                .try_parse_function_call(span, None, &ident)
                .merge_to_result()
                .map(|el| Statement::from_function_call(el, self.current_is(&SEMI_COLON))),
        }
    }

    pub(crate) fn default_methode_type(&self, modifier: TypeModifier, span: Span) -> SoulType {
        match &self.current_this {
            Some(val) => val.clone().with_modifier(Some(modifier)),
            None => SoulType::none(span).with_modifier(Some(modifier)),
        }
    }

    pub(crate) fn parse_extern_function(&mut self) -> SoulResult<Statement> {
        self.expect_ident(KeyWord::Extern.as_str())?;

        let string_literal = match &self.token().kind {
            TokenKind::StringLiteral(val) => val,
            other => {
                return Err(SoulError::new(
                    format!(
                        "expected string_literal of language name but got {}",
                        other.display()
                    ),
                    SoulErrorKind::InvalidIdent,
                    Some(self.token().span),
                ));
            }
        };

        let external = match string_literal.as_str() {
            "C" => ExternLanguage::C,
            _ => {
                return Err(SoulError::new(
                    format!("language {} is not supported", string_literal),
                    SoulErrorKind::InvalidIdent,
                    Some(self.token().span),
                ));
            }
        };

        self.bump();
        let name = self.try_bump_consume_ident()?;

        let span = self.token().span;
        match self.try_parse_function_signature(span, SoulType::none(span), name, Some(external)) {
            Ok(signature) => Ok(Statement::from_external_function(signature)),
            Err(TryError::IsErr(err)) => Err(err),
            Err(TryError::IsNotValue((_, err))) => Err(*err),
        }
    }

    pub(crate) fn try_parse_function_declaration(
        &mut self,
        start_span: Span,
        methode_type: SoulType,
        name: Ident,
    ) -> FuncResult<Spanned<Function>> {
        self.inner_function_declaration(start_span, methode_type, name, None)
    }

    fn inner_function_declaration(
        &mut self,
        start_span: Span,
        methode_type: SoulType,
        name: Ident,
        external: Option<ExternLanguage>,
    ) -> FuncResult<Spanned<Function>> {
        let begin = self.current_position();
        let modifier = methode_type.modifier.unwrap_or(TypeModifier::Mut);

        let signature =
            self.try_parse_function_signature(start_span, methode_type, name, external)?;

        let block = match self.parse_block(modifier) {
            Ok(val) => val,
            Err(err) => {
                if signature.node.parameters.is_empty() {
                    self.go_to(begin);
                    return TryNotValue((signature.node.name, Box::new(err)));
                } else {
                    return TryErr(err);
                }
            }
        };

        let span = signature.span;
        Ok(Spanned::new(
            Function { block, signature },
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

        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_define()?
        } else {
            vec![]
        };

        if self.current_is(&CURLY_OPEN) {
            // could be struct constructor
            return TryNotValue(self.get_expect_error(&CURLY_OPEN));
        }

        let arguments = self.parse_arguments().try_err()?;
        TryOk(Spanned::new(
            FunctionCall {
                generics,
                id: None,
                arguments,
                resolved: None,
                name: name.clone(),
                callee: callee.map(|expr| Box::new(expr.clone())),
            },
            self.span_combine(start_span),
        ))
    }

    pub(crate) fn try_parse_function_signature(
        &mut self,
        start_span: Span,
        methode_type: SoulType,
        name: Ident,
        external: Option<ExternLanguage>,
    ) -> FuncResult<Spanned<FunctionSignature>> {
        let begin_position = self.current_position();
        let result = self.inner_parse_function_signature(start_span, methode_type, name, external);

        if result.is_err() {
            self.go_to(begin_position);
        }

        result
    }

    pub(crate) fn parse_generic_define(&mut self) -> TryResult<Vec<SoulType>, SoulError> {
        self.expect(&ARROW_LEFT).try_err()?;
        let mut types = vec![];
        loop {
            let ty = self.try_parse_type()?;
            types.push(ty);

            if self.current_is(&ARROW_RIGHT) {
                self.bump();
                break;
            }

            self.expect(&COMMA).try_err()?;
        }
        TryOk(types)
    }

    fn parse_arguments(&mut self) -> SoulResult<Vec<Argument>> {
        self.expect(&ROUND_OPEN)?;
        if self.current_is(&ROUND_CLOSE) {
            self.bump();
            return Ok(vec![]);
        }

        let mut values = vec![];
        loop {
            let name = if self.peek().kind == COLON {
                let name = self.try_bump_consume_ident()?;
                self.expect(&COLON)?;
                Some(name)
            } else {
                None
            };

            let value = self.parse_expression(&[COMMA, ROUND_CLOSE])?;
            values.push(Argument { name, value });
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
        external: Option<ExternLanguage>,
    ) -> FuncResult<Spanned<FunctionSignature>> {
        if !self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return TryNotValue((
                name,
                Box::new(self.get_expect_any_error(&[ROUND_OPEN, ARROW_LEFT])),
            ));
        }

        let generics = self.parse_generic_declare().try_err()?.unwrap_or(vec![]);

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
            id: None,
            generics,
            external,
            parameters,
            return_type,
            methode_type,
            function_kind,
        };

        TryOk(Spanned::new(signature, self.span_combine(start_span)))
    }

    pub fn parse_generic_declare(&mut self) -> SoulResult<Option<Vec<Generic>>> {
        if !self.current_is(&ARROW_LEFT) {
            return Ok(None);
        }

        self.bump();
        let mut generics = vec![];
        loop {
            let name = self.try_bump_consume_ident()?;
            generics.push(Generic { name });

            if self.current_is(&ARROW_RIGHT) {
                self.bump();
                return Ok(Some(generics));
            }
            self.expect(&COMMA)?;
        }
    }
}
