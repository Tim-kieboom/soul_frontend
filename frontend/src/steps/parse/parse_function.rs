use crate::{
    steps::parse::{
        parse_statement::{ARROW_LEFT, COLON, ROUND_OPEN},
        parser::Parser,
    },
    utils::try_result::{ResultTryResult, TryErr, TryError, TryNotValue, TryOk, TryResult},
};
use models::{
    abstract_syntax_tree::{
        expression::{BoxExpression, Expression, ExpressionKind},
        function::{Function, FunctionCall, FunctionCallee, FunctionSignature},
        soul_type::{GenericParameter, SoulType, TypeGeneric},
        spanned::Spanned,
        statment::{Ident, Statement, StatementKind},
    },
    error::{SoulError, SoulErrorKind, SoulResult, Span},
    soul_names::TypeModifier,
};

impl<'a> Parser<'a> {
    pub(crate) fn parse_function_call(
        &mut self,
        start_span: Span,
        callee: Option<BoxExpression>,
        name: String,
    ) -> SoulResult<Expression> {
        if !self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return Err(self.get_error_expected_any(&[ROUND_OPEN, ARROW_LEFT]));
        }

        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_type()?
        } else {
            vec![]
        };

        let arguments = self.parse_tuple()?;

        Ok(Expression::new(
            ExpressionKind::FunctionCall(FunctionCall {
                name,
                callee,
                generics,
                arguments,
            }),
            self.new_span(start_span),
        ))
    }

    pub(crate) fn try_parse_function_signature(
        &mut self,
        start_span: Span,
        modifier: TypeModifier,
        extention_type: Option<SoulType>,
        name: Ident,
    ) -> TryResult<Spanned<FunctionSignature>, (Ident, SoulError)> {
        let begin_position = self.current_position();

        if !self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return TryErr(self.get_error_expected_any(&[ROUND_OPEN, ARROW_LEFT]));
        }

        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_declare().try_err()?
        } else {
            vec![]
        };

        if !self.current_is(&ROUND_OPEN) {
            return TryErr(SoulError::new(
                format!(
                    "'{}' should be '{}'",
                    self.token().kind.display(),
                    ROUND_OPEN.display()
                ),
                SoulErrorKind::InvalidTokenKind,
                Some(self.new_span(start_span)),
            ));
        }

        const IS_FUNCTION: bool = true;
        let (parameters, this) = match self.parse_named_tuple_type(IS_FUNCTION) {
            Ok(val) => val,
            Err(TryError::IsNotValue(err)) => {
                self.go_to(begin_position);
                return TryNotValue((name, err));
            }
            Err(TryError::IsErr(err)) => return TryErr(err),
        };

        let return_type = if self.current_is(&COLON) {
            self.bump();
            match self.try_parse_type() {
                Ok(val) => val,
                Err(TryError::IsErr(err)) => return TryErr(err),
                Err(TryError::IsNotValue(err)) => return TryNotValue((name, err)),
            }
        } else {
            SoulType::none()
        };

        let callee = extention_type.map(|ty| {
            Spanned::new(
                FunctionCallee {
                    extention_type: ty,
                    this,
                },
                self.token().span,
            )
        });

        let signature = FunctionSignature {
            name,
            callee,
            generics,
            modifier,
            parameters,
            return_type,
        };

        TryOk(Spanned::new(signature, self.new_span(start_span)))
    }

    pub(crate) fn try_parse_function_declaration(
        &mut self,
        start_span: Span,
        modifier: TypeModifier,
        extention_type: Option<SoulType>,
        name: Ident,
    ) -> TryResult<Statement, (Ident, SoulError)> {
        let Spanned {
            node: signature,
            span,
            attributes,
        } = self.try_parse_function_signature(start_span, modifier, extention_type, name)?;
        let block = self
            .parse_block(modifier)
            .map_err(|err| TryError::IsErr(err))?;

        Ok(Statement::with_atribute(
            StatementKind::Function(Function { signature, block }),
            span.combine(self.token().span),
            attributes,
        ))
    }

    pub(crate) fn parse_generic_declare(&mut self) -> SoulResult<Vec<GenericParameter>> {
        todo!()
    }

    pub(crate) fn parse_generic_type(&mut self) -> SoulResult<Vec<TypeGeneric>> {
        todo!()
    }
}
