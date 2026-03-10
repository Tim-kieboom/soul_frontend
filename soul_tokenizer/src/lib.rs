pub use token::{Number, Token, TokenKind};
pub use token_stream::{TokenStream, TokenStreamPosition};

mod lexer;
mod symbolkind_from_lexer;
mod token;
mod token_stream;

#[cfg(test)]
mod tests;

pub fn to_token_stream<'a>(source: &'a str) -> TokenStream<'a> {
    TokenStream::new(source)
}
