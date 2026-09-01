use ast_model::{
    expression::Binding,
    soul_type::SoulType,
    statements::{Statement, VarPattern, Variable},
};
use soul_tokenizer::model::TokenKind;
use soul_utils::{
    Ident, TypeModifier, collections::try_result::{
        ResultMapNotValue, ResultTryErr, ToResult, TryErr, TryError, TryNotValue, TryOk, TryResult,
    }, error::SoulResult, fault::Fault, soul_names::Symbol, span::Span,
};

use crate::{
    parse::statements::{try_assign_type, variable::AssignType}, parser::Parser, utils::{ARROW_LEFT, COLON, CONST, CURLY_OPEN, MUT, ROUND_OPEN, SEMI_COLON, STAMENT_END_TOKENS},
};

impl<'a, 'f> Parser<'a, 'f> {
    pub(super) fn try_parse_from_mut(
        &mut self,
        start_span: Span
    ) -> SoulResult<Statement> {
        self.expect(&MUT)?;
        let modifier = TypeModifier::Mut;

        let name = self.try_bump_consume_ident()?;
        let pattern = if name.as_str() == "_" {
            VarPattern::Discard
        } else {
            VarPattern::Simple {
                modifier,
                binding: Binding::new(self.alloc_node(), name),
            }
        };

        let mut ty = None;
        if self.current_is(&COLON) {
            self.bump();
            ty = Some(self.try_parse_type().merge_to_result()?);
        }

        if self.current_is_any(STAMENT_END_TOKENS) {
            let span = self.token().span;
            let variable = Variable {
                id: self.alloc_node(),
                is_public: false,
                pattern,
                ty,
                modifier,
                initialize_value: None,
            };
            return Ok(Statement::new_variable(variable, span));
        }

        let assign = match &self.token().kind {
            TokenKind::Symbol(val) if AssignType::from_symbool(*val).is_some() => {
                AssignType::from_symbool(*val).unwrap()
            }
            _ => return Err(self.invalid_assign()),
        };

        if assign != AssignType::Assign && assign != AssignType::Declaration {
            return Err(self.invalid_assign());
        }

        self.bump();
        let value = self.parse_expression_id(STAMENT_END_TOKENS)?;
        let variable = Variable {
            id: self.alloc_node(),
            is_public: false,
            pattern,
            ty,
            modifier,
            initialize_value: Some(value),
        };

        Ok(Statement::new_variable(
            variable,
            self.span_combine(start_span),
        ))
    } 

    pub(super) fn try_parse_from_const(
        &mut self,
        start_span: Span,
    ) -> SoulResult<Statement> {
        self.expect(&CONST)?;
        let modifier = TypeModifier::Const;
        const IS_CONST: bool = true;

        if self.current_is(&ROUND_OPEN) {
            let pattern = self.parse_tuple_pattern()?;
            return self.parse_pattern_declaration(pattern, modifier, start_span);
        }

        if self.current_is(&CURLY_OPEN) {
            return self.try_parse_named_tuple_or_block(modifier, start_span)
                .merge_to_result();
        }

        let name = self.try_bump_consume_ident()?;
        match &self.token().kind {
            &CURLY_OPEN => {
                self.try_parse_constructor_declaration(name, modifier, start_span)
            },
            &ROUND_OPEN | &ARROW_LEFT => {
                self
                    .try_parse_function_declaration_id(start_span, &SoulType::None, IS_CONST, name)
                    .map(Statement::from_function)
                    .map_try_not_value(|(_, err)| err)
                    .merge_to_result()
            },
            TokenKind::Symbol(Symbol::DoubleColon) => {
                self.bump();
                let value = self.parse_expression_id(STAMENT_END_TOKENS)?;
                Ok(Statement::new_variable(
                    Variable {
                        id: self.alloc_node(),
                        is_public: false,
                        pattern: VarPattern::Simple {
                            modifier,
                            binding: Binding::new(self.alloc_node(), name),
                        },
                        ty: None,
                        modifier,
                        initialize_value: Some(value),
                    },
                    self.span_combine(start_span),
                ))
            },
            _ => {
                let pattern = VarPattern::Simple {
                    modifier,
                    binding: Binding::new(self.alloc_node(), name),
                };
                let assign = match &self.token().kind {
                    TokenKind::Symbol(val) if AssignType::from_symbool(*val).is_some() => {
                        AssignType::from_symbool(*val).unwrap()
                    }
                    _ => return Err(self.invalid_assign()),
                };
                if assign != AssignType::Assign && assign != AssignType::Declaration {
                    return Err(self.invalid_assign());
                }
                self.bump();
                let value = self.parse_expression_id(STAMENT_END_TOKENS)?;
                Ok(Statement::new_variable(
                    Variable {
                        id: self.alloc_node(),
                        is_public: false,
                        pattern,
                        ty: None,
                        modifier,
                        initialize_value: Some(value),
                    },
                    self.span_combine(start_span),
                ))
            }
        }
    }

    /// Try named-tuple destructuring `{field1, field2} = expr` or fall back to block.
    fn try_parse_named_tuple_or_block(
        &mut self,
        modifier: TypeModifier,
        start_span: Span,
    ) -> TryResult<Statement, Fault> {
        let saved = self.tokens.current_position();

        match self.try_parse_named_tuple(start_span, modifier) {
            Ok(val) => return TryOk(val),
            Err(TryError::IsErr(err)) => return TryErr(err),
            Err(TryError::IsNotValue(())) => (),
        }

        self.goto(saved);
        let block = self.parse_block(modifier).try_err()?;
        let span = self.span_combine(start_span);
        let semicolon = self.current_is(&SEMI_COLON);
        TryOk(Statement::new_block(
            &mut self.forest.store,
            block,
            span,
            semicolon,
        ))
    }

    fn try_parse_named_tuple(
        &mut self,
        start_span: Span,
        modifier: TypeModifier,
    ) -> TryResult<Statement, ()> {
        if modifier != TypeModifier::Const {
            return TryNotValue(());
        }

        let Ok(pattern) = self.parse_named_tuple_pattern() else {
            return TryNotValue(());
        };

        let Some(assign) = try_assign_type(&self.token()) else {
            return TryNotValue(());
        };

        if assign == AssignType::Assign || assign == AssignType::Declaration {
            self.bump();
            return self
                .parse_expression_id(STAMENT_END_TOKENS)
                .map(|value| {
                    Statement::new_variable(
                        Variable {
                            id: self.alloc_node(),
                            is_public: false,
                            pattern,
                            ty: None,
                            modifier,
                            initialize_value: Some(value),
                        },
                        self.span_combine(start_span),
                    )
                })
                .try_err();
        }

        return TryNotValue(());
    }

    /// Try constructor destructuring: `TypeName{field1, field2} = expr`.
    fn try_parse_constructor_declaration(
        &mut self,
        type_name: Ident,
        modifier: TypeModifier,
        start_span: Span,
    ) -> SoulResult<Statement> {
        let pattern = self.parse_constructor_pattern(type_name)?;
        let assign = match &self.token().kind {
            TokenKind::Symbol(val) if AssignType::from_symbool(*val).is_some() => {
                AssignType::from_symbool(*val).unwrap()
            }
            _ => {
                return Err(Fault::error(
                    "expected '=' or ':=' after constructor pattern",
                    Some(self.token().span),
                ));
            }
        };

        if assign != AssignType::Assign && assign != AssignType::Declaration {
            return Err(Fault::error(
                format!(
                    "'{}' is not valid for variable declaration (can use ['=', ':='])",
                    assign.as_str()
                ),
                Some(self.token().span),
            ));
        }

        self.bump();
        let value = self.parse_expression_id(STAMENT_END_TOKENS)?;
        Ok(Statement::new_variable(
            Variable {
                id: self.alloc_node(),
                is_public: false,
                pattern,
                ty: None,
                modifier,
                initialize_value: Some(value),
            },
            self.span_combine(start_span),
        ))
    }

    /// Parse a declaration with the given pattern-fn, checking for = or :=.
    fn parse_pattern_declaration(
        &mut self,
        pattern: VarPattern,
        modifier: TypeModifier,
        start_span: Span,
    ) -> SoulResult<Statement> {
        let assign = match &self.token().kind {
            TokenKind::Symbol(val) if AssignType::from_symbool(*val).is_some() => {
                AssignType::from_symbool(*val).unwrap()
            }
            _ => {
                return Err(Fault::error(
                    "expected '=' or ':=' after destructuring pattern",
                    Some(self.token().span),
                ));
            }
        };

        if assign != AssignType::Assign && assign != AssignType::Declaration {
            return Err(Fault::error(
                format!(
                    "'{}' is not valid for variable declaration (can use ['=', ':='])",
                    assign.as_str()
                ),
                Some(self.token().span),
            ));
        }

        self.bump();
        let value = self.parse_expression_id(STAMENT_END_TOKENS)?;
        Ok(Statement::new_variable(
            Variable {
                id: self.alloc_node(),
                is_public: false,
                pattern,
                ty: None,
                modifier,
                initialize_value: Some(value),
            },
            self.span_combine(start_span),
        ))
    }

    fn invalid_assign(&self) -> Fault {
        Fault::error(
            format!("'{}' should be '=' or ':='", self.token().kind.display(),),
            Some(self.token().span),
        )
    }
}
