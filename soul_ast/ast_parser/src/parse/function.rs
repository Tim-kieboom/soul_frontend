use ast_model::{
    FunctionKind,
    expression::{Argument, ExpressionId, FunctionCall},
    soul_type::{Generic, SoulType},
    statements::{
        ExternLanguage, ExternalFunction, Function, FunctionSignature, FunctionThisKind, Parameter,
        Statement,
    },
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    FunctionId, Ident, TypeModifier,
    collections::try_result::{
        ResultTryErr, ResultTryNotValue, ToResult, TryErr, TryError, TryNotValue, TryOk, TryResult,
    },
    error::SoulResult,
    fault::Fault,
    literal::{StringLiteral, TokenLiteral},
    span::{Span, Spanned},
};

use crate::{
    parser::Parser,
    utils::{
        ARROW_LEFT, ARROW_RIGHT, ASSIGN, COLON, COMMA, CONST_REF, CURLY_OPEN, MUT_REF, ROUND_CLOSE,
        ROUND_OPEN, SEMI_COLON,
    },
};

type FuncResult<T> = TryResult<T, (Ident, Fault)>;
impl<'a, 'f> Parser<'a, 'f> {
    
    pub(crate) fn parse_any_function(&mut self) -> SoulResult<Statement> {
        let ident = self.try_bump_consume_ident()?;
        let modifier = self.try_bump_type_modiffier().unwrap_or(TypeModifier::Mut);

        let span = self.token().span;
        match self.try_parse_function_declaration(
            span,
            modifier,
            self.current_this.clone().unwrap_or(SoulType::None),
            ident,
        ) {
            Ok(val) => Ok(Statement::from_function(val)),
            Err(TryError::IsErr(err)) => Err(err),
            Err(TryError::IsNotValue((ident, _err))) => self
                .try_parse_function_call(span, None, &ident)
                .merge_to_result()
                .map(|el| Statement::from_function_call(self.store, el, self.current_is(&SEMI_COLON))),
        }
    }

    pub(crate) fn try_parse_function_call(
        &mut self,
        start_span: Span,
        callee: Option<ExpressionId>,
        name: &Ident,
    ) -> TryResult<Spanned<FunctionCall>, Fault> {
        if !self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return TryNotValue(self.get_expect_any_error(&[ROUND_OPEN, ARROW_LEFT]));
        }

        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_define()?
        } else {
            vec![]
        };

        self.try_parse_function_call_generic(start_span, callee, generics, name)
    }

    pub(crate) fn try_parse_function_call_generic(
        &mut self,
        start_span: Span,
        callee: Option<ExpressionId>,
        generics: Vec<SoulType>,
        name: &Ident,
    ) -> TryResult<Spanned<FunctionCall>, Fault> {
        let start_position = self.tokens.current_position();

        if !self.current_is(&ROUND_OPEN) {
            self.go_to(start_position);
            return TryNotValue(self.get_expect_error(&CURLY_OPEN));
        }

        let arguments = self.parse_arguments().try_err()?;
        TryOk(Spanned::new(
            FunctionCall {
                callee,
                generics,
                id: None,
                arguments,
                resolved: None,
                name: name.clone(),
            },
            self.span_combine(start_span),
        ))
    }

    pub(crate) fn try_parse_function_declaration(
        &mut self,
        start_span: Span,
        modifier: TypeModifier,
        methode_type: SoulType,
        name: Ident,
    ) -> FuncResult<Spanned<FunctionId>> {
        self.inner_function_declaration(start_span, modifier, methode_type, name, None)
            .map(|spanned| {
                spanned.map(|function| self.store.insert_function(FunctionKind::Normal(function)))
            })
    }

    pub(crate) fn parse_extern_function(&mut self) -> SoulResult<Statement> {
        self.expect(&TokenKind::Keyword(KeyWord::Extern))?;

        let string_literal = match &self.token().kind {
            TokenKind::Literal(TokenLiteral::String(val)) => val,
            other => {
                return Err(Fault::error(
                    format!(
                        "expected string_literal of language name but got {}",
                        other.display()
                    ),
                    Some(self.token().span),
                ));
            }
        };

        let normal_string = match string_literal {
            StringLiteral::Str(val) => val,
            other => {
                let tag = other.to_tag().expect("is not normal so should have tag");
                return Err(Fault::error(
                    format!(
                        "expected normal string_literal of language name but got {tag:?} string_literl",
                    ),
                    Some(self.token().span),
                ));
            }
        };

        let external = match normal_string.as_str() {
            "C" => ExternLanguage::C,
            _ => {
                return Err(Fault::error(
                    format!("language {normal_string} is not supported"),
                    Some(self.token().span),
                ));
            }
        };

        self.bump();
        let name = self.try_bump_consume_ident()?;

        let span = self.token().span;
        match self.try_parse_function_signature(
            span,
            TypeModifier::Mut,
            SoulType::None,
            name,
            Some(external),
        ) {
            Ok(signature) => {
                let span = signature.span;
                let id = self
                    .store
                    .insert_function(FunctionKind::External(ExternalFunction { signature }));
                Ok(Statement::from_external_function(Spanned::new(id, span)))
            }
            Err(TryError::IsErr(err)) => Err(err),
            Err(TryError::IsNotValue((_, err))) => Err(err),
        }
    }

    pub(crate) fn try_parse_function_signature(
        &mut self,
        start_span: Span,
        modifier: TypeModifier,
        methode_type: SoulType,
        name: Ident,
        external: Option<ExternLanguage>,
    ) -> FuncResult<Spanned<FunctionSignature>> {
        let begin_position = self.tokens.current_position();
        let result =
            self.inner_parse_function_signature(start_span, modifier, methode_type, name, external);

        if result.is_err() {
            self.go_to(begin_position);
        }

        result
    }

    pub(crate) fn try_parse_parameters(
        &mut self,
    ) -> TryResult<(Vec<Parameter>, FunctionThisKind), Fault> {
        let begin = self.tokens.current_position();

        let result = self.inner_parameters();
        if result.is_err() {
            self.go_to(begin);
        }

        result
    }

    pub fn parse_generic_declare(&mut self) -> SoulResult<Option<Vec<Generic>>> {
        if !self.current_is(&ARROW_LEFT) {
            return Ok(None);
        }

        self.bump();
        let mut generics = vec![];
        loop {
            let name = self.try_bump_consume_ident()?;
            let bound = if self.current_is(&COLON) {
                self.bump();
                Some(self.try_parse_type().merge_to_result()?)
            } else {
                None
            };
            generics.push(Generic { name, bound });

            if self.current_is(&ARROW_RIGHT) {
                self.bump();
                return Ok(Some(generics));
            }
            self.expect(&COMMA)?;
        }
    }

    pub(crate) fn parse_arguments(&mut self) -> SoulResult<Vec<Argument>> {
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

            let value = self.parse_expression_id(&[COMMA, ROUND_CLOSE])?;
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
        modifier: TypeModifier,
        method_type: SoulType,
        name: Ident,
        external: Option<ExternLanguage>,
    ) -> FuncResult<Spanned<FunctionSignature>> {
        if !self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return TryNotValue((name, self.get_expect_any_error(&[ROUND_OPEN, ARROW_LEFT])));
        }

        let generics = self.parse_generic_declare().try_err()?.unwrap_or(vec![]);

        if !self.current_is(&ROUND_OPEN) {
            return TryErr(self.get_expect_error(&ROUND_OPEN));
        }

        let (parameters, function_kind) = match self.try_parse_parameters() {
            Ok(val) => val,
            Err(TryError::IsErr(err)) => return TryErr(err),
            Err(TryError::IsNotValue(err)) => return TryNotValue((name, err)),
        };

        let return_type = match self.current_is(&COLON) {
            true => {
                self.bump();
                match self.try_parse_type() {
                    Ok(val) => val,
                    Err(TryError::IsErr(err)) => return TryErr(err),
                    Err(TryError::IsNotValue(err)) => return TryNotValue((name, err)),
                }
            }
            false => SoulType::None,
        };

        let signature = FunctionSignature {
            name,
            external,
            modifier,
            generics,
            parameters,
            return_type,
            method_type,
            function_kind,
            id: self.store.alloc_function(),
        };

        TryOk(Spanned::new(signature, self.span_combine(start_span)))
    }

    fn inner_function_declaration(
        &mut self,
        start_span: Span,
        modifier: TypeModifier,
        methode_type: SoulType,
        name: Ident,
        external: Option<ExternLanguage>,
    ) -> FuncResult<Spanned<Function>> {
        let signature =
            self.try_parse_function_signature(start_span, modifier, methode_type, name, external)?;

        let block = match self.parse_block(TypeModifier::Mut) {
            Ok(val) => val,
            Err(err) => {
                if signature.value.parameters.is_empty() {
                    return TryNotValue((signature.value.name, err));
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

    fn inner_parameters(&mut self) -> TryResult<(Vec<Parameter>, FunctionThisKind), Fault> {
        self.expect(&ROUND_OPEN).try_err()?;

        let mut types = vec![];
        let mut function_kind = FunctionThisKind::Static;

        let mut has_default = false;
        loop {
            self.skip_end_lines();
            if self.current_is(&ROUND_CLOSE) {
                break;
            }

            match self.inner_parameter_this(&mut function_kind)? {
                Loop::None => (),
                Loop::Break => break,
                Loop::Continue => continue,
            }

            let modifier = self
                .try_bump_type_modiffier()
                .unwrap_or(TypeModifier::Const);

            let name = self.try_bump_consume_ident().try_not_value()?;

            if !self.current_is(&COLON) {
                // is probebly tuple
                return Err(TryError::IsNotValue(self.get_expect_error(&COLON)));
            }
            self.bump();

            let ty = self.try_parse_type()?; // if not value is probebly named_tuple expression

            let default = if self.current_is(&ASSIGN) {
                self.bump();
                has_default = true;
                Some(self.parse_expression_id(&[COMMA, ROUND_CLOSE]).try_err()?)
            } else {
                None
            };

            if default.is_none() && has_default {
                self.log_error(
                    "you can not have a non default parameter after default parameter",
                    Some(name.span()),
                );
            }

            types.push(Parameter {
                ty,
                name,
                default,
                modifier,
                node_id: None,
            });

            self.skip_end_lines();
            if self.current_is(&ROUND_CLOSE) {
                break;
            }

            self.expect(&COMMA).try_err()?;
        }

        self.expect(&ROUND_CLOSE).try_err()?;

        Ok((types, function_kind))
    }

    fn inner_parameter_this(&mut self, kind: &mut FunctionThisKind) -> TryResult<Loop, Fault> {
        let this = match &self.token().kind {
            &CONST_REF => {
                self.bump();
                Some(FunctionThisKind::ConstRef)
            }
            &MUT_REF => {
                self.bump();
                Some(FunctionThisKind::MutRef)
            }
            TokenKind::Ident(val) if val == "this" => Some(FunctionThisKind::Consume),
            _ => None,
        };

        if let Some(callee) = this {
            if *kind != FunctionThisKind::Static {
                return TryErr(Fault::error(
                    "can not have more then one 'this' in methode",
                    Some(self.token().span),
                ));
            }

            *kind = callee;
            self.expect_ident("this").try_not_value()?;

            return match self.token().kind {
                ROUND_CLOSE => TryOk(Loop::Break),
                COMMA => {
                    self.bump();
                    TryOk(Loop::Continue)
                }
                _ => TryErr(self.get_expect_any_error(&[COMMA, ROUND_CLOSE])),
            };
        }

        Ok(Loop::None)
    }
}

enum Loop {
    None,
    Break,
    Continue,
}
