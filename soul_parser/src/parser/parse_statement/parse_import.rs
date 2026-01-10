use parser_models::ast::{Import, Statement, StatementKind};
use soul_tokenizer::TokenKind;
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_import_path::SoulImportPath,
    soul_names::KeyWord,
    symbool_kind::SymbolKind,
};

use crate::parser::{
    Parser,
    parse_utils::{COMMA, SQUARE_CLOSE, SQUARE_OPEN},
};

impl<'a> Parser<'a> {
    pub(super) fn parse_import(&mut self) -> SoulResult<Statement> {
        const PATH_SYMBOL: TokenKind = TokenKind::Symbol(SymbolKind::DoubleColon);

        self.expect_ident(KeyWord::Import.as_str())?;
        self.expect(&SQUARE_OPEN)?;

        let mut paths = vec![];
        let mut bases = vec![];
        let mut current = SoulImportPath::new();
        loop {
            self.skip_end_lines();

            match &self.token().kind {
                TokenKind::Ident(name) => current.push(name),
                &SQUARE_CLOSE => break,
                other => {
                    return Err(SoulError::new(
                        format!("expected ident or ']' got '{}'", other.display()),
                        SoulErrorKind::InvalidTokenKind,
                        Some(self.token().span),
                    ));
                }
            }

            self.bump();

            if self.current_is(&PATH_SYMBOL) {
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
                current = bases.last().cloned().unwrap_or(SoulImportPath::new());
                continue;
            }

            if self.current_is(&SQUARE_CLOSE) && !bases.is_empty() {
                paths.push(current);

                let _ = bases.pop();
                self.bump();

                while !bases.is_empty() && !self.current_is(&SQUARE_CLOSE) {
                    self.skip_end_lines();
                    self.expect(&SQUARE_CLOSE)?;
                    let _ = bases.pop();
                }

                current = bases.last().cloned().unwrap_or(SoulImportPath::new());
                continue;
            }

            if self.current_is(&SQUARE_CLOSE) {
                paths.push(current);
                break;
            }

            return Err(SoulError::new(
                format!(
                    "'{}' unexpected for import statement",
                    self.token().kind.display(),
                ),
                SoulErrorKind::InvalidTokenKind,
                Some(self.token().span),
            ));
        }

        self.expect(&SQUARE_CLOSE)?;
        Ok(Statement::new(
            StatementKind::Import(Import{id: None, paths}),
            self.token().span,
        ))
    }
}
