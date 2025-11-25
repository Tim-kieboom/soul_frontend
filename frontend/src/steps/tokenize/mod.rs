use crate::steps::tokenize::token_stream::TokenStream;

pub mod from_lexer;
pub mod tokenizer;
pub mod token_stream;

#[cfg(test)]
mod test_tokenizer;

#[derive(Debug)]
pub(crate) struct Request<'a> {
    pub source: &'a str
}

#[derive(Debug, Clone)]
pub(crate) struct Response<'a> {
    pub token_stream: TokenStream<'a>,
}