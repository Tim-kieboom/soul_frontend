use anyhow::Result;
use soul_utils::literal::{Literal, Number, StringLiteral};
use std::io::Write;
use soul_tokenizer::{TokenStream, model::TokenKind};

use crate::display::display_span;

pub(crate) fn display_tokens<'a>(tokens: TokenStream<'a>, writer: &mut impl Write) -> Result<()> {
    
    for token in tokens {
        let token = token.map_err(|err| anyhow::Error::msg(err.to_string()))?;
        writer.write("Span(".as_bytes())?;
        display_span(token.span, writer)?;
        
        let tab = if token.span.is_single() {"\t\t\t"} else {"\t\t"};
        writer.write_fmt(format_args!("){tab}>> "))?;
        match token.kind {
            TokenKind::Literal(literal) => literal.display(writer)?,
            TokenKind::Keyword(key_word) => writer.write_fmt(format_args!("{}", key_word.as_str()))?,
            TokenKind::Symbol(symbol) => writer.write_fmt(format_args!("'{}'", symbol.as_str()))?,
            TokenKind::Types(types) => writer.write_fmt(format_args!("{}", types.as_str()))?,
            TokenKind::Ident(ident) => writer.write_fmt(format_args!("{ident:?}"))?,
            TokenKind::EndLine => _ = writer.write("'\\n'".as_bytes())?,
            TokenKind::EndFile => _ = writer.write("<end-file>".as_bytes())?,
        };
        writer.write("\n".as_bytes())?;
    }
    Ok(())
}

trait Display {
    fn display(&self, writer: &mut impl Write) -> std::io::Result<()>;
}
impl Display for Literal {
    fn display(&self, writer: &mut impl Write) -> std::io::Result<()> {
        match self {
            Self::StringLiteral(str) => match str {
                StringLiteral::Str(str) => writer.write_fmt(format_args!("str({str:?})")),
                StringLiteral::Cstr(str) => writer.write_fmt(format_args!("cstr({str:?})")),
            },
            Self::Number(num) => match num {
                Number::Int(n) => writer.write_fmt(format_args!("{n}")),
                Number::Uint(n) => writer.write_fmt(format_args!("{n}")),
                Number::Float(n) => writer.write_fmt(format_args!("{n}")),
            },
            Self::Char(ch) => writer.write_fmt(format_args!("char({ch:?})")),
        }
    }
}