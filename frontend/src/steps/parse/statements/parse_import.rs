use models::{abstract_syntax_tree::{spanned::Spanned, statment::{Statement, StatementKind}}, error::{SoulError, SoulErrorKind, SoulResult}, soul_names::KeyWord, soul_page_path::SoulPagePath, symbool_kind::SymboolKind};

use crate::steps::{parse::{COMMA, SQUARE_CLOSE, SQUARE_OPEN, parser::Parser}, tokenize::token_stream::TokenKind};

impl<'a> Parser<'a> {
    pub(crate) fn parse_import(&mut self) -> SoulResult<Spanned<StatementKind>> {
        const PATH_SYMBOOL: TokenKind = TokenKind::Symbool(SymboolKind::DoubleColon);

        self.expect_ident(KeyWord::Import.as_str())?;
        self.expect(&SQUARE_OPEN)?;

        let mut paths = vec![];
        let mut bases = vec![];
        let mut current = SoulPagePath::new();
        loop {
            self.skip_end_lines();

            match &self.token().kind {
                TokenKind::Ident(name) => current.push(name),
                &SQUARE_CLOSE => break,
                other => {
                    return Err(SoulError::new(
                        format!("expected ident got '{}'", other.display()),
                        SoulErrorKind::InvalidTokenKind,
                        Some(self.token().span),
                    ));
                }
            };

            self.bump();

            if self.current_is(&PATH_SYMBOOL) {
                self.bump();
                if self.current_is(&SQUARE_OPEN) {
                    self.bump();
                    bases.push(current.clone());
                }

                continue;
            }

            if self.current_is(&COMMA) {
                self.bump();
                self.skip_end_lines();

                paths.push(current);
                current = bases.last().cloned().unwrap_or(SoulPagePath::new());

                continue;
            }

            if self.current_is(&SQUARE_CLOSE) && !bases.is_empty() {
                paths.push(current);

                let _ = bases.pop().is_none();
                self.bump();
                
                while !bases.is_empty() && !self.current_is(&SQUARE_CLOSE) {
                    self.skip_end_lines();
                    self.expect(&SQUARE_CLOSE)?;
                    let _ = bases.pop();
                }

                current = bases.last().cloned().unwrap_or(SoulPagePath::new());

                continue;
            }

            if self.current_is(&SQUARE_CLOSE) {
                paths.push(current);
                break;
            }

            return Err(SoulError::new(
                format!(
                    "'{}' unexpected for import statment",
                    self.token().kind.display()
                ),
                SoulErrorKind::InvalidTokenKind,
                Some(self.token().span),
            ));
        }

        self.expect(&SQUARE_CLOSE)?;
        Ok(Statement::new(
            StatementKind::Import(paths),
            self.token().span,
        ))
    }
}