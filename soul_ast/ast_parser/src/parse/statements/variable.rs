use ast_model::statements::{Statement, Variable};
use soul_tokenizer::model::TokenKind;
use soul_utils::{
    TypeModifier, collections::try_result::ToResult, define_symbols, error::SoulResult,
    fault::Fault, soul_names::Symbol,
};

use crate::{
    parser::Parser,
    utils::{ASSIGN, COLON, COLON_ASSIGN, STAMENT_END_TOKENS},
};

impl<'a, 'f> Parser<'a, 'f> {
    pub(crate) fn parse_variable(&mut self) -> SoulResult<Statement> {
        const DEFAULT_MODIFIER: TypeModifier = TypeModifier::Const;
        let modifier = self.try_bump_type_modiffier().unwrap_or(DEFAULT_MODIFIER);

        let name = self.try_bump_consume_ident()?;
        let name_span = name.span();

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

        if ty.is_some() && assign_type.is_none() {
            return Err(self.get_expect_any_error(&[COLON_ASSIGN, ASSIGN]));
        }

        let assign_type = match assign_type {
            Some(val) => val,
            None => {
                return Ok(Statement::new_variable(
                    Variable {
                        ty,
                        name,
                        modifier,
                        node_id: None,
                        initialize_value: None,
                    },
                    self.span_combine(name_span),
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
                ty,
                name,
                modifier,
                node_id: None,
                initialize_value: Some(self.parse_expression_id(STAMENT_END_TOKENS)?),
            },
            self.span_combine(name_span),
        ))
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
