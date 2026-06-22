use std::path::PathBuf;

use ast_model::statements::{Import, ImportItem, ImportKind, ImportPath, Statement, StatementKind};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::{
    Ident, collections::soul_import_path::SoulImportPath, error::SoulResult, fault::Fault, soul_names::Symbol
};

use crate::{
    parser::Parser,
    utils::{AS_STR, COMMA, CURLY_CLOSE, CURLY_OPEN, IMPORT, ROUND_CLOSE, ROUND_OPEN, STAR},
};

impl<'a, 'f> Parser<'a, 'f> {
    pub(super) fn parse_import(&mut self) -> SoulResult<Statement> {
        let start_span = self.token().span;

        let mut spans = vec![];
        let mut paths = vec![];
        self.expect(&IMPORT)?;
        if self.current_is(&ROUND_OPEN) {
            self.bump();
            self.skip_end_lines();
            loop {
                if self.current_is(&ROUND_CLOSE) {
                    break;
                }

                let span = self.token().span;
                paths.push(self.inner_parse_import()?);
                spans.push(self.span_combine(span));

                self.skip_end_lines();
            }

            self.expect(&ROUND_CLOSE)?;
        } else {
            let span = self.token().span;
            paths.push(self.inner_parse_import()?);
            spans.push(self.span_combine(span));
        }

        for (i, path) in paths.iter().enumerate() {
            if path.module.is_external() {
                continue
            }

            self.parse_child_module(path, spans[i]);
        }

        let import = Import { paths };

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

        Ok(ImportPath {
            module: path,
            kind,
            lib_name,
        })
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
                    return Err(Fault::error(
                        "expected ',' or '}' in import list".to_string(),
                        Some(self.token().span),
                    ));
                }
            }
        }
        Ok((this, this_alias, items))
    }

    fn parse_import_path(&mut self) -> SoulResult<(SoulImportPath, Option<String>)> {
        const IS_EXTERNAL: bool = true;
        const IS_INTERNAL: bool = false;
        const THIS_PORJECT: TokenKind = TokenKind::Keyword(KeyWord::Crate);
        const SEPARATOR: TokenKind = TokenKind::Symbol(Symbol::Dot);
        const PREV_SUPER: TokenKind = TokenKind::Symbol(Symbol::Slash);

        let mut lib_name = None;
        let mut path = SoulImportPath::new(PathBuf::default(), IS_EXTERNAL);
        if self.current_is(&THIS_PORJECT) {
            let current_path = self.source_path.clone();
            path = SoulImportPath::new(current_path, IS_INTERNAL);
            path.set_absolute();
            self.bump();
            self.expect(&SEPARATOR)?;
        } else if self.current_is(&SEPARATOR) {
            let mut current_path = self.current_path().to_path_buf();
            self.bump();

            while self.current_is(&PREV_SUPER) {
                self.bump();
                if !current_path.pop() {
                    return Err(Fault::error(
                        "could not pop path",
                        Some(self.token().span),
                    ));
                }

                self.expect(&SEPARATOR)?;
            }

            path = SoulImportPath::new(current_path, IS_INTERNAL);
        } else if let TokenKind::Ident(name) = &self.token().kind {
            lib_name = Some(name.clone());
        } else {
            self.log_error(
                format!("'{}' not allowed in import", self.token().kind.display()),
                Some(self.token().span),
            );
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

        if !self.current_is(&TokenKind::EndFile) {
            self.expect(&TokenKind::EndLine)?;
        }
        Ok((path, lib_name))
    }

    fn is_non_path_import_symbool(&self) -> bool {
        const TOKENS: &[TokenKind] = &[CURLY_OPEN, STAR];

        self.current_is_ident(KeyWord::As.as_str()) || self.current_is_any(TOKENS)
    }
}
