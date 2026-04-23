use ast::{Import, ImportKind, ImportPath, Statement};
use ast::{ImportItem, StatementKind};
use soul_tokenizer::TokenKind;
use soul_utils::Ident;
use soul_utils::symbool_kind::SymbolKind;
use soul_utils::{
    error::{SoulError, SoulErrorKind, SoulResult},
    soul_import_path::SoulImportPath,
    soul_names::KeyWord,
};

use crate::parser::parse_utils::{AS_STR, COMMA, CURLY_OPEN, ROUND_CLOSE, ROUND_OPEN, STAR};
use crate::parser::{Parser, parse_utils::CURLY_CLOSE};

impl<'a, 'f> Parser<'a, 'f> {
    pub(super) fn parse_import(&mut self) -> SoulResult<Statement> {
        let start_span = self.token().span;

        let mut paths = vec![];
        self.expect_ident(KeyWord::Import.as_str())?;
        if self.current_is(&ROUND_OPEN) {
            self.bump();
            self.skip_end_lines();
            loop {
                if self.current_is(&ROUND_CLOSE) {
                    break;
                }
                paths.push(self.inner_parse_import()?);

                self.skip_end_lines();
            }

            self.expect(&ROUND_CLOSE)?;
        } else {
            paths.push(self.inner_parse_import()?);
        }

        let import = Import { id: None, paths };

        self.expect(&TokenKind::EndLine)?;

        Ok(Statement::new(
            StatementKind::Import(import),
            start_span.combine(self.token().span),
        ))
    }

    fn inner_parse_import(&mut self) -> SoulResult<ImportPath> {
        
        let (path, lib_name) = self.parse_import_path()?;
        let kind = match &self.token().kind {
            &CURLY_OPEN => {
                self.bump();
                let (this, this_alias, items) = self.parse_import_items()?;
                self.expect(&CURLY_CLOSE)?;
                ImportKind::Items {
                    this,
                    this_alias,
                    items,
                }
            }
            &STAR => {
                self.bump();
                ImportKind::Glob
            }
            TokenKind::Ident(ident) => match ident.as_str() {
                AS_STR => {
                    self.bump();
                    let alias = self.try_bump_consume_ident()?.into();
                    ImportKind::Alias(alias)
                }
                _ => ImportKind::Module,
            },
            _ => ImportKind::Module,
        };

        Ok(ImportPath { module: path, kind, lib_name })
    }

    fn parse_import_items(&mut self) -> SoulResult<(bool, Option<Ident>, Vec<ImportItem>)> {
        let mut this = false;
        let mut items = vec![];
        let mut this_alias = None;
        loop {
            let name = self.try_bump_consume_ident()?;
            if name.as_str() == "this" {
                this = true;
                if self.current_is_ident(KeyWord::As.as_str()) {
                    self.bump();
                    let alias = self.try_bump_consume_ident()?;
                    this_alias = Some(alias);
                }
            } else if self.current_is_ident(KeyWord::As.as_str()) {
                self.bump();
                let alias = self.try_bump_consume_ident()?;
                items.push(ImportItem::Alias { name, alias })
            } else {
                items.push(ImportItem::Normal(name))
            };

            match self.token().kind {
                COMMA => {
                    self.bump();
                }
                CURLY_CLOSE => {
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
        Ok((this, this_alias, items))
    }

    fn parse_import_path(&mut self) -> SoulResult<(SoulImportPath, Option<String>)> {
        const THIS_PORJECT: &str = KeyWord::Crate.as_str();
        const SEPARATOR: TokenKind = TokenKind::Symbol(SymbolKind::Dot);
        const PREV_SUPER: TokenKind = TokenKind::Symbol(SymbolKind::Slash);

        let mut lib_name = None;
        let mut path = SoulImportPath::new();
        if self.current_is_ident(THIS_PORJECT) {
            let current_path = self.context.source_folder.clone();
            path = SoulImportPath::from(current_path);
            self.bump();
            self.expect(&SEPARATOR)?;
        } else if self.current_is(&SEPARATOR) {
            let mut current_path = self.context.current_path().clone();
            self.bump();

            while self.current_is(&PREV_SUPER) {
                self.bump();
                if !current_path.pop() {
                    return Err(SoulError::new(
                        "could not pop path",
                        SoulErrorKind::PathNotFound,
                        Some(self.token().span),
                    ));
                }

                self.expect(&SEPARATOR)?;
            }

            path = SoulImportPath::from(current_path);
        } else if let TokenKind::Ident(name) = &self.token().kind {
            lib_name = Some(name.clone());
        } else {
            self.log_error(SoulError::new(format!("'{}' not allowed in import", self.token().kind.display()), SoulErrorKind::InvalidContext, Some(self.token().span)));
        }

        loop {
            if self.is_non_path_import_symbool() {
                return Ok((path, lib_name));
            }

            let ident = self.try_bump_consume_ident()?;
            path.push(ident.as_str());

            if !self.current_is(&SEPARATOR) {
                break;
            }

            self.bump();
        }

        self.expect(&TokenKind::EndLine)?;
        Ok((path, lib_name))
    }

    fn is_non_path_import_symbool(&self) -> bool {
        const TOKENS: &[TokenKind] = &[CURLY_OPEN, STAR];

        self.current_is_ident(KeyWord::As.as_str()) || self.current_is_any(TOKENS)
    }
}
