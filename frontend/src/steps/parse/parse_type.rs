use crate::steps::{parse::{parse_statement::{COLON, COMMA}, parser::{Parser, TryError, TryResult}}, tokenize::token_stream::TokenKind};
use models::{abstract_syntax_tree::soul_type::{NamedTupleType, SoulType}, error::{SoulError, SoulErrorKind, SoulResult}};

impl<'a> Parser<'a> {
    
    pub(crate) fn parse_type(&mut self) -> SoulResult<SoulType> {
        todo!()
    }
    
    pub(crate) fn parse_named_tuple_type(&mut self, open: &TokenKind, close: &TokenKind) -> TryResult<NamedTupleType, SoulError> {

        self.expect(open)
            .map_err(|err| TryError::IsErr(err))?;

        if self.current_is(close) {
            self.bump();
            return Ok(
                NamedTupleType{types: vec![]}
            ) 
        }

        let mut types = vec![];
        let begin_position = self.current_position();

        loop {
            
            let token = self.bump_consume();
            let name = match token.kind {
                TokenKind::Ident(val) => val,
                other => return Err(TryError::IsNotValue(
                    SoulError::new(
                        format!("'{}' should be ident", other.display()),
                        SoulErrorKind::InvalidTokenKind,
                        Some(self.token().span),
                    )
                )),
            };

            if !self.current_is(&COLON) {
                self.go_to(begin_position);
                return Err(TryError::IsNotValue(self.get_error_expected(&COLON))) // is probebly tuple
            }

            let ty = self.parse_type()
                .map_err(|err| TryError::IsNotValue(err))?; // is probebly named_tuple expression 
            
            if self.current_is(close) {
                break
            }

            types.push((name, ty));
            self.expect(&COMMA)
                .map_err(|err| TryError::IsErr(err))?;
        
        }

        self.expect(close)
            .map_err(|err| TryError::IsErr(err))?;
        
        Ok(
            NamedTupleType{types}
        )
    }
}