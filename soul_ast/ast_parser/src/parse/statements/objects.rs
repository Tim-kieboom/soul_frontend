use ast_model::{
    soul_type::{SoulType, Stub},
    statements::{Field, Statement, StatementKind, Struct},
};
use soul_tokenizer::model::{TokenKind, keyword::KeyWord};
use soul_utils::error::SoulResult;

use crate::{
    parser::Parser,
    utils::{CURLY_CLOSE, CURLY_OPEN},
};

impl<'a, 'f> Parser<'a, 'f> {
    pub fn parse_struct(&mut self) -> SoulResult<Statement> {
        let struct_span = self.token().span;
        self.expect(&TokenKind::Keyword(KeyWord::Struct))?;
        let struct_name = self.try_bump_consume_ident()?;
        let generics = self.parse_generic_declare()?.unwrap_or(vec![]);

        self.expect(&CURLY_OPEN)?;
        self.skip_end_lines();

        let mut fields = vec![];
        let mut statements = vec![];
        let this_type = SoulType::Stub(Stub::new(struct_name.to_string()));

        let prev_type = self.current.this_type.take();
        self.current.this_type = Some(this_type);
        loop {
            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }
            let start_span = self.token().span;
            let statement = self.parse_statement()?;
            let is_public = statement.is_public();
            match statement.node {
                StatementKind::Variable(variable) => {
                    fields.push(Field::new(variable, is_public));
                }

                StatementKind::Enum(_)
                | StatementKind::Trait(_)
                | StatementKind::Import(_)
                | StatementKind::Struct(_)
                | StatementKind::TypeDef(_)
                | StatementKind::Function(_)
                | StatementKind::ExternalFunction(_) => {
                    let id = self.store.insert_statement(statement);
                    statements.push(id)
                }

                StatementKind::UseBlock(_)
                | StatementKind::Assignment(_)
                | StatementKind::Expression { .. } => {
                    self.log_error(
                        format!(
                            "{} can not be used in struct body",
                            statement.node.variant_name()
                        ),
                        Some(self.span_combine(start_span)),
                    );
                    continue;
                }
            }

            self.skip_end_lines();
            if self.current_is(&CURLY_CLOSE) {
                break;
            }
        }
        self.current.this_type = prev_type;

        self.expect(&CURLY_CLOSE)?;

        let struct_ = Struct {
            id: None,
            fields,
            generics,
            statements,
            name: struct_name,
        };

        Ok(Statement::new(
            StatementKind::Struct(struct_),
            self.span_combine(struct_span),
        ))
    }
}
