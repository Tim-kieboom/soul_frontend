use crate::steps::parse::parser::Parser;

pub mod parser;

mod expect;
mod parse_conditionals;
mod parse_expression;
mod parse_function;
mod parse_group_expression;
mod parse_objects;
mod parse_statement;
mod parse_type;

pub type Request<'a> = crate::steps::tokenize::Response<'a>;

#[derive(Debug)]
pub(crate) struct Response<'a> {
    pub parser: Parser<'a>,
}
