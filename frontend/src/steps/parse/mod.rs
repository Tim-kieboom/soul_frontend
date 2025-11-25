use crate::steps::{parse::parser::Parser};

pub mod parser;

mod expect;
mod parse_type;
mod parse_function;
mod parse_statement;
mod parse_expression;
mod parse_group_expression;

pub type Request<'a> = crate::steps::tokenize::Response<'a>;

#[derive(Debug)]
pub(crate) struct Response<'a> {
    pub parser: Parser<'a>
}