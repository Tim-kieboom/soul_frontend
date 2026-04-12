use soul_utils::span::ModuleId;
pub use token::{Number, Token, TokenKind};
pub use token_stream::{TokenStream, TokenStreamPosition};

mod lexer;
mod symbolkind_from_lexer;
mod token;
mod token_stream;

#[cfg(test)]
mod tests;

/// Converts source code into a token stream for parsing.
pub fn to_token_stream<'a>(source: &'a str, module: ModuleId) -> TokenStream<'a> {
    TokenStream::new(source, module)
}
