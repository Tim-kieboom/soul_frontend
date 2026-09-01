use ast_model::{
    FunctionKind,
    block::Block,
    expression::{Argument, Constructor, Expression, ExpressionKind, FunctionCall, FunctionCallee},
    soul_type::{ArrayKind, ArrayType, Generic, SoulType},
    statements::{
        ExternLanguage, Function, FunctionModifier, FunctionSignature, FunctionSignatureHelper,
        FunctionThisKind, InnerFunctionSignature, Parameter, Statement,
    },
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    FunctionId, Ident, TypeModifier,
    collections::try_result::{
        ResultMapNotValue, ResultTryErr, ResultTryNotValue, ToResult, TryErr, TryError,
        TryNotValue, TryOk, TryResult,
    },
    error::SoulResult,
    fault::Fault,
    literal::{StringLiteral, TokenLiteral},
    soul_names::Symbol,
    span::{Span, Spanned},
};

use crate::{
    parser::Parser,
    utils::{
        ARRAY, ARROW_LEFT, ARROW_RIGHT, ASSIGN, COLON, COMMA, CONST, CURLY_OPEN, DOT,
        DOUBLE_QUESTION, LAMBDA_ARROW, OPTIONAL, REF, ROUND_CLOSE, ROUND_OPEN, SEMI_COLON,
        SQUARE_CLOSE, SQUARE_OPEN, STAMENT_END_TOKENS,
    },
};
const CONTRUCTOR_STR: &str = "This__ctor";
const ARRAY_CONTRUCTOR_STR: &str = "This__arrayCtor";

pub struct FuncError {
    pub ident: Ident,
    pub fault: Fault,
}
impl FuncError {
    pub fn new(ident: Ident, fault: Fault) -> Box<Self> {
        Box::new(Self { ident, fault })
    }
}

type FuncResult<T> = TryResult<T, Box<FuncError>>;
impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_any_function(&mut self) -> SoulResult<Statement> {
        let is_const = self.current_is(&CONST);
        if is_const {
            self.bump();
        }

        let ident = self.try_bump_consume_ident()?;

        let span = self.token().span;

        let this_type = self.current.this_type.take().unwrap_or(SoulType::None);
        let result = self.try_parse_function_declaration_id(span, &this_type, is_const, ident);
        self.current.this_type = Some(this_type);

        match result {
            Ok(val) => Ok(Statement::from_function(val)),
            Err(TryError::IsErr(err)) => Err(err),
            Err(TryError::IsNotValue(err)) => self
                .try_parse_function_call(span, None, &err.ident)
                .merge_to_result()
                .map(|expression| {
                    let id = self.forest.store.insert_expression(expression);
                    Statement::from_expression(&self.forest.store, id, self.current_is(&SEMI_COLON))
                }),
        }
    }

    pub(crate) fn try_parse_function_call(
        &mut self,
        start_span: Span,
        callee: Option<FunctionCallee>,
        name: &Ident,
    ) -> TryResult<Expression, Fault> {
        if !self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return TryNotValue(self.get_expect_any_error(&[ROUND_OPEN, ARROW_LEFT]));
        }

        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_define()?
        } else {
            vec![]
        };

        if self.current_is(&DOT) {
            if callee.is_some() {
                return TryErr(Fault::error(
                    format!("`{}` invalid", Symbol::Dot.as_str()),
                    Some(self.span_combine(start_span)),
                ));
            }

            self.bump();
            if self.current_is_any(&[SQUARE_OPEN, ARRAY]) {
                let collection_type = self.type_from_ident(name.clone(), generics);
                let array = self.parse_array(Some(collection_type)).try_err()?;
                return TryOk(Expression::from_any_array(array));
            }

            if self.current_is(&ROUND_OPEN) {
                let arguments = self.parse_arguments().try_err()?;
                let ty = self.type_from_ident(name.clone(), generics);
                let ctor = Constructor {
                    id: self.alloc_node(),
                    ty,
                    arguments,
                };
                return TryOk(Expression::new(
                    ExpressionKind::Constructor(ctor),
                    self.span_combine(start_span),
                ));
            }

            return TryErr(Fault::error(
                "expected array literal or '(' after type constructor",
                Some(self.token().span),
            ));
        }

        let call = self.try_parse_function_call_generic(start_span, callee, generics, name)?;
        match &self.token().kind {
            &DOT | &DOUBLE_QUESTION | &SQUARE_OPEN => {
                let primary = Expression::from_function_call(call);
                self.parse_primary_expression(primary, STAMENT_END_TOKENS)
                    .try_err()
            }
            _ => TryOk(Expression::from_function_call(call)),
        }
    }

    pub(crate) fn try_parse_function_call_generic(
        &mut self,
        start_span: Span,
        callee: Option<FunctionCallee>,
        generics: Vec<SoulType>,
        ident: &Ident,
    ) -> TryResult<Spanned<FunctionCall>, Fault> {
        let start_position = self.tokens.current_position();

        if !self.current_is(&ROUND_OPEN) {
            self.goto(start_position);
            return TryNotValue(self.get_expect_error(&CURLY_OPEN));
        }

        let arguments = self.parse_arguments().try_err()?;
        let optional_map = self.current_is(&OPTIONAL);
        if optional_map {
            self.bump();
            if !self.current_is(&DOT) {
                return TryErr(self.get_expect_error(&DOT));
            }
        }
        TryOk(Spanned::new(
            FunctionCall {
                callee,
                generics,
                id: self.alloc_node(),
                arguments,
                optional_map,
                name: ident.clone(),
            },
            self.span_combine(start_span),
        ))
    }

    pub(crate) fn try_parse_function_declaration_id(
        &mut self,
        start_span: Span,
        methode_type: &SoulType,
        is_const: bool,
        name: Ident,
    ) -> FuncResult<Spanned<FunctionId>> {
        self.try_parse_function_declaration(start_span, methode_type, is_const, name)
            .map(|spanned| {
                spanned.map(|function| {
                    self.forest
                        .store
                        .insert_function(FunctionKind::Normal(function))
                })
            })
    }

    pub(crate) fn try_parse_function_declaration(
        &mut self,
        start_span: Span,
        methode_type: &SoulType,
        is_const: bool,
        name: Ident,
    ) -> FuncResult<Spanned<Function>> {
        let position = self.tokens.current_position();
        match self.inner_function_declaration(start_span, methode_type, name, is_const, None) {
            Ok(spanned) => Ok(spanned),
            Err(err) => {
                self.goto(position);
                Err(err)
            }
        }
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
        match self.try_parse_function_signature(span, &SoulType::None, name, false, Some(external))
        {
            Ok(signature) => {
                let span = signature.span;
                let id = self
                    .forest
                    .store
                    .insert_function(FunctionKind::Signature(signature));
                Ok(Statement::from_external_function(Spanned::new(id, span)))
            }
            Err(TryError::IsErr(err)) => Err(err),
            Err(TryError::IsNotValue(err)) => Err(err.fault),
        }
    }

    pub(crate) fn try_parse_function_signature(
        &mut self,
        start_span: Span,
        methode_type: &SoulType,
        name: Ident,
        is_const: bool,
        external: Option<ExternLanguage>,
    ) -> FuncResult<Box<Spanned<InnerFunctionSignature>>> {
        let begin_position = self.tokens.current_position();
        let result =
            self.inner_parse_function_signature(start_span, methode_type, name, is_const, external);

        if result.is_err() {
            self.goto(begin_position);
        }

        result
    }

    pub(crate) fn try_parse_parameters(
        &mut self,
    ) -> TryResult<(Vec<Parameter>, FunctionThisKind), Fault> {
        let begin = self.tokens.current_position();

        let result = self.inner_parameters();
        if result.is_err() {
            self.goto(begin);
        }

        result
    }

    pub(crate) fn parse_generic_declare(&mut self) -> SoulResult<Option<Vec<Generic>>> {
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

    pub(crate) fn parse_function_contructor(
        &mut self,
        methode_type: &SoulType,
        is_const: bool,
    ) -> SoulResult<Spanned<Function>> {
        let start_span = self.token().span;
        self.expect(&DOT)?;
        match self.token().kind {
            ROUND_OPEN => {
                let name = Ident::new(CONTRUCTOR_STR, start_span);
                let mut methode = self
                    .try_parse_function_declaration(start_span, methode_type, is_const, name)
                    .map_try_not_value(|err| err.fault)
                    .merge_to_result()?
                    .value;

                let signature = &mut methode.signature.value;
                if signature.function_kind != FunctionThisKind::Static {
                    self.log_error(
                        "`This.(..)` has to be static function",
                        Some(signature.name.span()),
                    );
                }
                signature.function_kind = FunctionThisKind::Ctor;
                signature.return_type = methode_type.clone();

                Ok(Spanned::new(methode, self.span_combine(start_span)))
            }
            SQUARE_OPEN => {
                let name = Ident::new(ARRAY_CONTRUCTOR_STR, start_span);
                self.bump();
                let mut array_type = self.try_parse_type().merge_to_result()?;
                array_type = SoulType::Array(ArrayType {
                    of_type: Box::new(array_type),
                    kind: ArrayKind::StackArrayWildcard,
                });
                self.expect(&SQUARE_CLOSE)?;
                self.expect(&ROUND_OPEN)?;
                let arg = self.try_bump_consume_ident()?;
                self.expect(&ROUND_CLOSE)?;
                let block = if self.current_is(&LAMBDA_ARROW) {
                    self.bump();
                    self.skip_end_lines();
                    let expression = self.parse_expression_id(STAMENT_END_TOKENS)?;
                    let statement = Statement::from_expression(
                        &self.forest.store,
                        expression,
                        self.current_is(&SEMI_COLON),
                    );
                    let statement = self.forest.store.insert_statement(statement);
                    self.forest.store.insert_block(Block {
                        statements: vec![statement],
                        span: self.span_combine(start_span),
                        is_const: false,
                    })
                } else {
                    self.parse_block(TypeModifier::Mut)?
                };

                let id = self.forest.store.alloc_function();
                let signature = InnerFunctionSignature {
                    id,
                    name,
                    modifier: FunctionModifier::empty(),
                    method_type: methode_type.clone(),
                    return_type: methode_type.clone(),
                    parameters: vec![Parameter {
                        name: arg,
                        default: None,
                        ty: array_type,
                        id: self.alloc_node(),
                        is_mut: false,
                    }],
                    generics: vec![],
                    function_kind: FunctionThisKind::ArrayCtor,
                    external: None,
                    node_id: self.alloc_node(),
                };

                let function = Function {
                    signature: FunctionSignature::with_span(
                        signature,
                        self.span_combine(start_span),
                    ),
                    block,
                };

                Ok(Spanned::new(function, self.span_combine(start_span)))
            }
            _ => Err(self.get_expect_any_error(&[ROUND_OPEN, SQUARE_OPEN])),
        }
    }

    fn inner_parse_function_signature(
        &mut self,
        start_span: Span,
        method_type: &SoulType,
        name: Ident,
        is_const: bool,
        external: Option<ExternLanguage>,
    ) -> FuncResult<Box<Spanned<InnerFunctionSignature>>> {
        if !self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return TryNotValue(FuncError::new(
                name,
                self.get_expect_any_error(&[ROUND_OPEN, ARROW_LEFT]),
            ));
        }

        let generics = match self.parse_generic_declare() {
            Ok(val) => val.unwrap_or(vec![]),
            Err(err) => return TryNotValue(FuncError::new(name, err)),
        };

        if !self.current_is(&ROUND_OPEN) {
            return TryErr(self.get_expect_error(&ROUND_OPEN));
        }

        let (parameters, function_kind) = match self.try_parse_parameters() {
            Ok(val) => val,
            Err(TryError::IsErr(err)) => return TryErr(err),
            Err(TryError::IsNotValue(err)) => return TryNotValue(FuncError::new(name, err)),
        };

        let return_type = match self.current_is(&COLON) {
            true => {
                self.bump();
                match self.try_parse_type() {
                    Ok(val) => val,
                    Err(TryError::IsErr(err)) => return TryErr(err),
                    Err(TryError::IsNotValue(err)) => {
                        return TryNotValue(FuncError::new(name, err));
                    }
                }
            }
            false => SoulType::None,
        };

        let generics = match self.parse_where_clause(generics) {
            Ok(val) => val,
            Err(err) => return TryNotValue(FuncError::new(name, err)),
        };

        let modifier = if is_const {
            FunctionModifier::CONST
        } else {
            FunctionModifier::default()
        };

        let signature = InnerFunctionSignature {
            node_id: self.alloc_node(),
            name,
            external,
            modifier,
            generics,
            parameters,
            return_type,
            function_kind,
            id: self.forest.store.alloc_function(),
            method_type: method_type.clone(),
        };

        let signature = Spanned::new(signature, self.span_combine(start_span));
        TryOk(Box::new(signature))
    }

    fn parse_where_clause(&mut self, mut generics: Vec<Generic>) -> SoulResult<Vec<Generic>> {
        if !self.current_is(&TokenKind::Keyword(KeyWord::GenericWhere)) {
            return Ok(generics);
        }
        self.bump();

        loop {
            self.skip_end_lines();

            let name = self.try_bump_consume_ident()?;
            self.expect(&COLON)?;
            let bound = self.try_parse_type().merge_to_result()?;

            match generics
                .iter_mut()
                .find(|g| g.name.as_str() == name.as_str())
            {
                Some(existing) => {
                    if existing.bound.is_none() {
                        existing.bound = Some(bound);
                    }
                }
                None => generics.push(Generic {
                    name,
                    bound: Some(bound),
                }),
            }

            self.skip_end_lines();
            if !self.current_is(&COMMA) {
                break;
            }
            self.bump();
        }

        Ok(generics)
    }

    fn inner_function_declaration(
        &mut self,
        start_span: Span,
        methode_type: &SoulType,
        name: Ident,
        is_const: bool,
        external: Option<ExternLanguage>,
    ) -> FuncResult<Spanned<Function>> {
        let signature =
            self.try_parse_function_signature(start_span, methode_type, name, is_const, external)?;

        let block = if self.current_is(&LAMBDA_ARROW) {
            self.bump();
            self.skip_end_lines();
            let expr = match self.parse_expression_id(STAMENT_END_TOKENS) {
                Ok(val) => val,
                Err(err) => return TryErr(err),
            };
            let statement =
                Statement::from_expression(&self.forest.store, expr, self.current_is(&SEMI_COLON));
            let statement = self.forest.store.insert_statement(statement);
            self.forest.store.insert_block(Block {
                statements: vec![statement],
                span: signature.span,
                is_const: false,
            })
        } else {
            match self.parse_block(TypeModifier::Mut) {
                Ok(val) => val,
                Err(err) => {
                    if signature.value.parameters.is_empty() {
                        return TryNotValue(FuncError::new(signature.value.name, err));
                    } else {
                        return TryErr(err);
                    }
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

            match self.inner_parameter_this(&mut function_kind, &mut types)? {
                Loop::None => (),
                Loop::Break => break,
                Loop::Continue => continue,
            }

            let modifier = self.try_bump_mut().unwrap_or(TypeModifier::Const);
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
                id: self.alloc_node(),
                ty,
                name,
                default,
                is_mut: modifier == TypeModifier::Mut,
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

    fn inner_parameter_this(
        &mut self,
        kind: &mut FunctionThisKind,
        types: &mut Vec<Parameter>,
    ) -> TryResult<Loop, Fault> {
        let mut is_ref_prefix = false;
        let this = match &self.token().kind {
            &REF => {
                is_ref_prefix = true;
                self.bump();
                if matches!(self.token().kind, TokenKind::Keyword(KeyWord::Mut)) {
                    self.bump();
                    Some(FunctionThisKind::MutRef)
                } else {
                    Some(FunctionThisKind::ConstRef)
                }
            }
            TokenKind::Ident(val) if val == "this" => Some(FunctionThisKind::Consume),
            _ => None,
        };

        if is_ref_prefix && !self.current_is_ident("this") {
            let is_mut = matches!(this, Some(FunctionThisKind::MutRef));
            let name = self.try_bump_consume_ident().try_not_value()?;
            if !self.current_is(&COLON) {
                return Err(TryError::IsNotValue(self.get_expect_error(&COLON)));
            }
            self.bump();
            let ty = self.try_parse_type()?;
            types.push(Parameter {
                id: self.alloc_node(),
                ty,
                name,
                default: None,
                is_mut,
            });

            self.skip_end_lines();
            return if self.current_is(&ROUND_CLOSE) {
                TryOk(Loop::Break)
            } else {
                self.expect(&COMMA).try_err()?;
                TryOk(Loop::Continue)
            };
        }

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
