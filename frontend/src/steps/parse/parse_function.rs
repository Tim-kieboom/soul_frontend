use models::{abstract_syntax_tree::{expression::{BoxExpression, Expression, ExpressionKind}, function::{Function, FunctionCall, FunctionCallee, FunctionSignature}, soul_type::{GenericParameter, SoulType, TypeGeneric}, spanned::Spanned, statment::{Ident, Statement, StatementKind}}, error::{SoulError, SoulErrorKind, SoulResult, Span}, soul_names::TypeModifier};

use crate::steps::parse::{parse_statement::{ARROW_LEFT, COLON, ROUND_CLOSE, ROUND_OPEN}, parser::{Parser, TryError, TryResult}};

impl<'a> Parser<'a> {
    
    pub(crate) fn parse_function_call(&mut self, start_span: Span, callee: Option<BoxExpression>, name: String) -> SoulResult<Expression> {
        if !self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return Err(self.get_error_expected_any(&[ROUND_OPEN, ARROW_LEFT]))
        }

        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_type()?
        } 
        else {
            vec![]
        };

        let arguments = self.parse_tuple()?;

        Ok(
            Expression::new(
                ExpressionKind::FunctionCall(FunctionCall{
                    name,
                    callee,
                    generics,
                    arguments,
                }), 
                self.new_span(start_span),
            )
        )
    }

    pub(crate) fn parse_function_declaration(&mut self, start_span: Span, modifier: TypeModifier, callee: Option<Spanned<FunctionCallee>>, name: Ident) -> TryResult<Statement, Ident> {
        
        let begin_position = self.current_position();
        
        if !self.current_is_any(&[ROUND_OPEN, ARROW_LEFT]) {
            return Err(TryError::IsErr(
                self.get_error_expected_any(&[ROUND_OPEN, ARROW_LEFT])
            ))
        }

        let generics = if self.current_is(&ARROW_LEFT) {
            self.parse_generic_declare()
                .map_err(|err| TryError::IsErr(err))?
        } 
        else {
            vec![]
        };

        if !self.current_is(&ROUND_OPEN) {
            
            return Err(
                TryError::IsErr(SoulError::new(
                    format!("'{}' should be '{}'", self.token().kind.display(), ROUND_OPEN.display()), 
                    SoulErrorKind::InvalidTokenKind, 
                    Some(self.new_span(start_span)),
                ))
            )
        }

        let parameters = match self.parse_named_tuple_type(&ROUND_OPEN, &ROUND_CLOSE) {
            Ok(val) => val,
            Err(TryError::IsNotValue(_)) => {
                self.go_to(begin_position);
                return Err(TryError::IsNotValue(name))
            },
            Err(TryError::IsErr(err)) => return Err(TryError::IsErr(err))
        };

        let return_type = if self.current_is(&COLON) {
            self.parse_type()
                .map_err(|err| TryError::IsErr(err))?
        }
        else {
            SoulType::none()
        };

        let signature = FunctionSignature {
            name,
            callee,
            generics,
            modifier,
            parameters,
            return_type,
        };

        let block = self.parse_block(modifier)
            .map_err(|err| TryError::IsErr(err))?;

        Ok(
            Statement::new(
                StatementKind::Function(Function{
                    signature,
                    block,
                }),
                self.new_span(start_span),
            )
        )
    }

    pub(crate) fn parse_generic_declare(&mut self) -> SoulResult<Vec<GenericParameter>> {
        todo!()
    }

    pub(crate) fn parse_generic_type(&mut self) -> SoulResult<Vec<TypeGeneric>> {
        todo!()
    }
}