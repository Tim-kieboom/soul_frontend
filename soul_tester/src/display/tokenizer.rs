use anyhow::Result;
use soul_tokenizer::{TokenStream, model::TokenKind};
use soul_utils::literal::{Number, StringLiteral, TokenLiteral};

use crate::display::writer::Writer;

pub(crate) fn display_tokens<'a>(tokens: TokenStream<'a>, writer: &mut impl Writer) -> Result<()> {
    for token in tokens {
        let token = token.map_err(|err| anyhow::Error::msg(format!("{err:?}")))?;
        let span_str = format!("{:?}", token.span);
        let tab = " ".repeat(30 - span_str.len());

        writer.push_fmt(format_args!("Span({span_str}){tab}>> ",))?;
        match token.kind {
            TokenKind::Literal(literal) => _ = literal.display(writer)?,
            TokenKind::Keyword(key_word) => {
                writer.push_fmt(format_args!("{}", key_word.as_str()))?
            }
            TokenKind::Symbol(symbol) => writer.push_fmt(format_args!("'{}'", symbol.as_str()))?,
            TokenKind::Types(types) => writer.push_fmt(format_args!("{}", types.as_str()))?,
            TokenKind::Ident(ident) => writer.push_fmt(format_args!("{ident:?}"))?,
            TokenKind::EndLine => _ = writer.push_str("'\\n'")?,
            TokenKind::EndFile => _ = writer.push_str("<end-file>")?,
        };
        writer.push_char('\n')?;
    }
    writer.writer_flush()
}

trait Display {
    fn display(&self, writer: &mut impl Writer) -> Result<()>;
}
impl Display for TokenLiteral {
    fn display(&self, writer: &mut impl Writer) -> Result<()> {
        match self {
            Self::String(str) => match str {
                StringLiteral::Str(str) => writer.push_fmt(format_args!("{str:?}")),
                StringLiteral::Cstr(str) => writer.push_fmt(format_args!("{str:?}")),
            },
            Self::Number(num) => match num {
                Number::Int(n) => writer.push_fmt(format_args!("{n}")),
                Number::Uint(n) => writer.push_fmt(format_args!("{n}")),
                Number::Float(n) => writer.push_fmt(format_args!("{n}")),
            },
            Self::Char(ch) => writer.push_fmt(format_args!("{ch:?}")),
        }
    }
}
