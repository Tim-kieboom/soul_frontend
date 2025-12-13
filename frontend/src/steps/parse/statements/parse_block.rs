use models::{abstract_syntax_tree::{block::Block, statment::UseBlock}, error::SoulResult, soul_names::{KeyWord, TypeModifier}};

use crate::{steps::{parse::{CURLY_CLOSE, CURLY_OPEN, SEMI_COLON, parser::Parser}, tokenize::token_stream::TokenKind}, utils::try_result::TryError};

impl<'a> Parser<'a> {
    pub(crate) fn parse_block(&mut self, modifier: TypeModifier) -> SoulResult<Block> {
        const END_TOKENS: &[TokenKind] = &[CURLY_CLOSE, TokenKind::EndFile];

        let mut statments = vec![];

        let scope_id = self.push_scope(modifier, None);

        self.expect(&CURLY_OPEN)?;
        while !self.current_is_any(END_TOKENS) {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }

            match self.parse_statement() {
                Ok(statment) => statments.push(statment),
                Err(err) => {
                    self.add_error(err);
                    self.skip_over_statement();
                }
            }

            self.skip_till(&[SEMI_COLON, TokenKind::EndLine]);
        }

        self.expect(&CURLY_CLOSE)?;
        Ok(Block {
            modifier,
            statments,
            scope_id,
        })
    }

    pub(crate) fn parse_use_block(&mut self) -> SoulResult<UseBlock> {
        self.expect_ident(KeyWord::Use.as_str())?;

        let ty = match self.try_parse_type() {
            Ok(val) => val,
            Err(TryError::IsErr(err)) => return Err(err),
            Err(TryError::IsNotValue(err)) => return Err(err),
        };

        let impl_trait = if self.current_is_ident(KeyWord::Impl.as_str()) {
            self.bump();

            match self.try_parse_type() {
                Ok(val) => Some(val),
                Err(TryError::IsErr(err)) => return Err(err),
                Err(TryError::IsNotValue(err)) => return Err(err),
            }
        } else {
            None
        };

        let block = self.parse_block(TypeModifier::Mut)?;
        Ok(UseBlock {
            impl_trait,
            ty,
            block,
        })
    }
}