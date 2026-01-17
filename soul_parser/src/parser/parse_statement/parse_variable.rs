use parser_models::ast::{Statement, StatementHelpers, VarTypeKind, Variable};
use soul_tokenizer::TokenKind;
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_names::{AssignType, TypeModifier},
    try_result::ToResult,
};

use crate::parser::{
    Parser,
    parse_utils::{ASSIGN, COLON, COLON_ASSIGN, STAMENT_END_TOKENS},
};

impl<'a> Parser<'a> {
    pub(crate) fn parse_variable(&mut self) -> SoulResult<Statement> {
        let name = self.try_bump_consume_ident()?;
        let name_span = name.get_span();

        const DEFAULT_MODIFIER: TypeModifier = TypeModifier::Const;
        let ty = if self.current_is(&COLON) {
            self.bump();
            let mut ty = self.try_parse_type().merge_to_result()?;

            if !self.current_is_any(&[COLON_ASSIGN, ASSIGN]) {
                return Err(self.get_expect_any_error(&[COLON_ASSIGN, ASSIGN]));
            }

            if ty.modifier.is_none() {
                ty.modifier = Some(DEFAULT_MODIFIER);
            }
            VarTypeKind::NonInveredType(ty)
        } else {
            VarTypeKind::InveredType(DEFAULT_MODIFIER)
        };

        let assign_type = match &self.token().kind {
            TokenKind::Symbol(kind) => AssignType::from_symbool(*kind),
            _ => None,
        };

        let assign_type = match assign_type {
            Some(val) => val,
            None => {
                return Ok(Statement::new_variable(
                    Variable {
                        ty,
                        name,
                        node_id: None,
                        initialize_value: None,
                    },
                    self.span_combine(name_span),
                ));
            }
        };

        if assign_type != AssignType::Declaration && assign_type != AssignType::Assign {
            return Err(SoulError::new(
                format!(
                    "'{}' is not valid for variable declaration (can use ['=', ':='])",
                    assign_type.as_str()
                ),
                SoulErrorKind::InvalidContext,
                Some(self.token().span),
            ));
        }

        self.bump();
        Ok(Statement::new_variable(
            Variable {
                ty,
                name,
                node_id: None,
                initialize_value: Some(self.parse_expression(STAMENT_END_TOKENS)?),
            },
            self.span_combine(name_span),
        ))
    }
}
