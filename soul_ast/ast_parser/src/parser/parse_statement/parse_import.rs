use ast::StatementKind;
use ast::{Import, ImportKind, ImportPath, Statement};
use soul_tokenizer::TokenKind;
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_import_path::SoulImportPath,
    soul_names::KeyWord,
};

use crate::parser::Parser;

impl<'a, 'f> Parser<'a, 'f> {
    pub(super) fn parse_import(&mut self) -> SoulResult<Statement> {
        let start_span = self.token().span;

        self.bump();

        let path = self.parse_import_path()?;

        let kind = if self.current_is_ident(KeyWord::As.as_str()) {
            self.bump();
            let alias = self.try_bump_consume_ident()?.into();
            ImportKind::Alias(alias)
        } else {
            ImportKind::This
        };

        let import_path = ImportPath { module: path, kind };

        let import = Import {
            id: None,
            paths: vec![import_path],
        };

        self.expect(&TokenKind::EndLine)?;

        Ok(Statement::new(
            StatementKind::Import(import),
            start_span.combine(self.token().span),
        ))
    }

    fn parse_import_path(&mut self) -> SoulResult<SoulImportPath> {
        match &self.token().kind {
            TokenKind::StringLiteral(path) => {
                let path = path.clone();
                self.bump();
                Ok(SoulImportPath::from_string(path))
            }
            TokenKind::Ident(ident) => {
                let import_name = ident.clone();
                self.bump();
                Ok(SoulImportPath::from_string(import_name))
            }
            _ => Err(SoulError::new(
                "expected import path".to_string(),
                SoulErrorKind::InvalidTokenKind,
                Some(self.token().span),
            )),
        }
    }
}
