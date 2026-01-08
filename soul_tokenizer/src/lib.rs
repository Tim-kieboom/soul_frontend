pub use token::{Token, TokenKind, Number};
pub use token_stream::{TokenStream, TokenStreamPosition};

mod lexer;
mod token;
mod token_stream;
mod symbolkind_from_lexer;

#[cfg(test)]
mod tests;

pub fn tokenize<'a>(source: &'a str) -> TokenStream<'a> {
    TokenStream::new(source)
}