use ast_model::{
    expression::Binding,
    statements::{
        NamedTuplePattern, Statement, TuplePattern, VarConstructorPattern, VarNamedPattern,
        VarPattern, Variable,
    },
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    Ident, TypeModifier, collections::try_result::ToResult, define_symbols, error::SoulResult,
    fault::Fault, soul_names::Symbol,
};

use crate::{
    parser::Parser,
    utils::{
        ASSIGN, COLON, COLON_ASSIGN, COMMA, CURLY_CLOSE, CURLY_OPEN, DOUBLE_DOT, ROUND_CLOSE,
        ROUND_OPEN, STAMENT_END_TOKENS,
    },
};

const MUT_STR: &str = KeyWord::Mut.as_str();

impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_variable(&mut self) -> SoulResult<Statement> {
        const DEFAULT_MODIFIER: TypeModifier = TypeModifier::Immut;
        let modifier = self.try_bump_mut().unwrap_or(DEFAULT_MODIFIER);
        let pattern_start = self.token().span;

        let pattern = self.parse_var_pattern(modifier)?;

        // Error: `mut` is not allowed on compound patterns
        if modifier != TypeModifier::Immut && !matches!(pattern, VarPattern::Simple { .. }) {
            return Err(Fault::error(
                format!("'{MUT_STR}' modifier cannot be applied to compound patterns; use per-binding '{MUT_STR}' instead (e.g., ({MUT_STR} a, b))", ),
                Some(pattern_start),
            ));
        }

        let ty = match self.current_is(&COLON) {
            true => {
                self.bump();
                Some(self.try_parse_type().merge_to_result()?)
            }
            false => None,
        };

        let assign_type = match &self.token().kind {
            TokenKind::Symbol(kind) => AssignType::from_symbool(*kind),
            _ => None,
        };

        if let TokenKind::Symbol(Symbol::DoubleColon) = self.token().kind {
            self.bump();
            let value = self.parse_expression_id(STAMENT_END_TOKENS)?;
            return Ok(Statement::new_variable(
                Variable {
                    id: self.alloc_node(),
                    is_public: false,
                    pattern,
                    ty,
                    modifier: TypeModifier::Const,
                    initialize_value: Some(value),
                },
                self.span_combine(pattern_start),
            ));
        }

        if ty.is_some() && assign_type.is_none() {
            return Err(self.get_expect_any_error(&[COLON_ASSIGN, ASSIGN]));
        }

        let assign_type = match assign_type {
            Some(val) => val,
            None => {
                return Ok(Statement::new_variable(
                    Variable {
                        id: self.alloc_node(),
                        is_public: false,
                        pattern,
                        ty,
                        modifier,
                        initialize_value: None,
                    },
                    self.span_combine(pattern_start),
                ));
            }
        };

        if assign_type != AssignType::Declaration && assign_type != AssignType::Assign {
            return Err(Fault::error(
                format!(
                    "'{}' is not valid for variable declaration (can use ['=', ':='])",
                    assign_type.as_str()
                ),
                Some(self.token().span),
            ));
        }

        self.bump();
        Ok(Statement::new_variable(
            Variable {
                id: self.alloc_node(),
                is_public: false,
                pattern,
                ty,
                modifier,
                initialize_value: Some(self.parse_expression_id(STAMENT_END_TOKENS)?),
            },
            self.span_combine(pattern_start),
        ))
    }

    /// Parse a pattern element, with an optional `mut` prefix.
    /// `default_modifier` is used when no explicit `mut` is found.
    pub(crate) fn parse_var_pattern(
        &mut self,
        default_modifier: TypeModifier,
    ) -> SoulResult<VarPattern> {
        let explicit_mod = self.try_bump_mut();
        let modifier = explicit_mod.unwrap_or(default_modifier);

        if self.current_is_ident("_") {
            self.bump();
            return Ok(VarPattern::Discard);
        }

        match &self.token().kind {
            TokenKind::Ident(_) => {
                let ident = self.try_bump_consume_ident()?;
                if self.current_is(&CURLY_OPEN) {
                    if explicit_mod.is_some() {
                        return Err(Fault::error(
                            "'mut' cannot be applied to constructor patterns; use per-field 'mut' instead".to_string(),
                            Some(ident.span()),
                        ));
                    }
                    return self.parse_constructor_pattern(ident);
                }
                Ok(VarPattern::Simple {
                    binding: Binding::new(self.alloc_node(), ident),
                    modifier,
                })
            }
            &ROUND_OPEN => {
                if explicit_mod.is_some() {
                    return Err(Fault::error(
                        "'mut' cannot be applied to tuple patterns; use per-element 'mut' instead (e.g., (mut a, b))"
                            .to_string(),
                        Some(self.token().span),
                    ));
                }
                self.parse_tuple_pattern()
            }
            &CURLY_OPEN => {
                if explicit_mod.is_some() {
                    return Err(Fault::error(
                        "'mut' cannot be applied to named-tuple patterns; use per-field 'mut' instead".to_string(),
                        Some(self.token().span),
                    ));
                }
                self.parse_named_tuple_pattern()
            }
            _ => Err(Fault::error(
                format!(
                    "expected variable name, `_`, `(`, or `{{` but found `{}`",
                    self.token().kind.display()
                ),
                Some(self.token().span),
            )),
        }
    }

    pub(crate) fn parse_tuple_pattern(&mut self) -> SoulResult<VarPattern> {
        self.expect(&ROUND_OPEN)?;
        let mut elements = Vec::new();
        let mut rest = false;

        let mut first = true;
        loop {
            self.skip_end_lines();
            if self.current_is(&ROUND_CLOSE) {
                break;
            }

            if !first {
                self.expect(&COMMA)?;
                self.skip_end_lines();
                if self.current_is(&ROUND_CLOSE) {
                    break;
                }
            }
            first = false;

            if self.current_is(&DOUBLE_DOT) {
                rest = true;
                self.bump();
                break;
            }

            elements.push(self.parse_var_pattern(TypeModifier::Const)?);
        }

        self.expect(&ROUND_CLOSE)?;
        Ok(VarPattern::Tuple(TuplePattern { elements, rest }))
    }

    pub(crate) fn parse_named_tuple_pattern(&mut self) -> SoulResult<VarPattern> {
        self.expect(&CURLY_OPEN)?;
        let mut fields = Vec::new();
        let mut rest = false;

        let mut first = true;
        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            if !first {
                self.expect(&COMMA)?;
                self.skip_end_lines();
                if self.current_is(&CURLY_CLOSE) {
                    break;
                }
            }
            first = false;

            if self.current_is(&DOUBLE_DOT) {
                rest = true;
                self.bump();
                break;
            }

            let modifier = self
                .try_bump_mut()
                .unwrap_or(TypeModifier::Immut);

            let field = self.try_bump_consume_ident()?;

            let binding = if self.current_is(&COLON) {
                self.bump();
                let alias = self.try_bump_consume_ident()?;
                if alias.as_str() == "_" {
                    None
                } else {
                    Some(Binding::new(self.alloc_node(), alias))
                }
            } else {
                Some(Binding::new(self.alloc_node(), field.clone()))
            };

            fields.push(VarNamedPattern {
                binding,
                field,
                modifier,
            });
        }

        self.expect(&CURLY_CLOSE)?;
        Ok(VarPattern::NamedTuple(NamedTuplePattern { fields, rest }))
    }

    pub(crate) fn parse_constructor_pattern(&mut self, type_name: Ident) -> SoulResult<VarPattern> {
        self.expect(&CURLY_OPEN)?;
        let mut fields = Vec::new();
        let mut rest = false;

        let mut first = true;
        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            if !first {
                self.expect(&COMMA)?;
                self.skip_end_lines();
                if self.current_is(&CURLY_CLOSE) {
                    break;
                }
            }
            first = false;

            if self.current_is(&DOUBLE_DOT) {
                rest = true;
                self.bump();
                break;
            }

            let modifier = self
                .try_bump_mut()
                .unwrap_or(TypeModifier::Immut);

            let field = self.try_bump_consume_ident()?;

            let binding = if self.current_is(&COLON) {
                self.bump();
                let alias = self.try_bump_consume_ident()?;
                if alias.as_str() == "_" {
                    None
                } else {
                    Some(Binding::new(self.alloc_node(), alias))
                }
            } else {
                Some(Binding::new(self.alloc_node(), field.clone()))
            };

            fields.push(VarNamedPattern {
                binding,
                field,
                modifier,
            });
        }

        self.expect(&CURLY_CLOSE)?;
        Ok(VarPattern::Constructor(VarConstructorPattern {
            type_name,
            fields,
            rest,
        }))
    }
}

define_symbols!(

    /// Assignment operators for variable assignment and modification.
    ///
    /// These operators are used to assign values to variables, with various
    /// compound assignment forms.
    pub enum AssignType {
        /// Declaration assignment (`:=`).
        Declaration => ":=", Symbol::ColonAssign,

        /// Simple assignment (`=`).
        Assign => "=", Symbol::Assign,
        AddAssign => "+=", Symbol::PlusEq,
        SubAssign => "-=", Symbol::MinusEq,
        MulAssign => "*=", Symbol::StarEq,
        DivAssign => "/=", Symbol::SlashEq,
        ModAssign => "%=", Symbol::ModEq,
        BitAndAssign => "&=", Symbol::AndEq,
        BitOrAssign => "|=", Symbol::OrEq,
        BitXorAssign => "^=", Symbol::XorEq,
    }
);
