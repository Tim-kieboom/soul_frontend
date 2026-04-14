use ast::StatementKind;
use ast::{Import, ImportKind, ImportPath, Statement};
use soul_tokenizer::TokenKind;
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_import_path::SoulImportPath,
    soul_names::KeyWord,
    Ident,
};

use crate::parser::{parse_utils::CURLY_CLOSE, Parser};

impl<'a, 'f> Parser<'a, 'f> {
    pub(super) fn parse_import(&mut self) -> SoulResult<Statement> {
        let start_span = self.token().span;

        self.bump();

        let path = self.parse_import_path()?;

        let kind = match self.token().kind {
            TokenKind::Symbol(sym) => match sym {
                soul_utils::symbool_kind::SymbolKind::CurlyOpen => {
                    self.bump();
                    let items = self.parse_import_items()?;
                    self.expect(&CURLY_CLOSE)?;
                    ImportKind::Items(items)
                }
                soul_utils::symbool_kind::SymbolKind::Star => {
                    self.bump();
                    ImportKind::Glob
                }
                _ => self.parse_import_kind_or_alias()?,
            },
            _ => self.parse_import_kind_or_alias()?,
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

    fn parse_import_items(&mut self) -> SoulResult<Vec<Ident>> {
        let mut items = vec![];
        loop {
            match &self.token().kind {
                TokenKind::Ident(_) => {
                    let ident = self.try_bump_consume_ident()?;
                    items.push(ident);
                }
                _ => {
                    return Err(SoulError::new(
                        "expected identifier in import list".to_string(),
                        SoulErrorKind::InvalidTokenKind,
                        Some(self.token().span),
                    ));
                }
            }

            match &self.token().kind {
                TokenKind::Symbol(soul_utils::symbool_kind::SymbolKind::Comma) => {
                    self.bump();
                }
                TokenKind::Symbol(soul_utils::symbool_kind::SymbolKind::CurlyClose) => {
                    break;
                }
                _ => {
                    return Err(SoulError::new(
                        "expected ',' or '}' in import list".to_string(),
                        SoulErrorKind::InvalidTokenKind,
                        Some(self.token().span),
                    ));
                }
            }
        }
        Ok(items)
    }

    fn parse_import_kind_or_alias(&mut self) -> SoulResult<ImportKind> {
        if self.current_is_ident(KeyWord::As.as_str()) {
            self.bump();
            let alias = self.try_bump_consume_ident()?.into();
            Ok(ImportKind::Alias(alias))
        } else {
            Ok(ImportKind::This)
        }
    }

    fn parse_import_path(&mut self) -> SoulResult<SoulImportPath> {
        let path = match &self.token().kind {
            TokenKind::StringLiteral(path) => SoulImportPath::from_str(path),
            TokenKind::Ident(ident) => SoulImportPath::from_str(ident),
            _ => {
                return Err(SoulError::new(
                    "expected import path".to_string(),
                    SoulErrorKind::InvalidTokenKind,
                    Some(self.token().span),
                ));
            }
        };

        self.bump();
        Ok(path)
    }
}
