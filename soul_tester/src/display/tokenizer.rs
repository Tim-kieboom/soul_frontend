use anyhow::Result;
use soul_tokenizer::{TokenStream, model::TokenKind};
use soul_utils::literal::{Number, StringLiteral, TokenLiteral};

use crate::{config, display::{write_create_file, writer::Writer}};

pub(crate) fn display_tokenizer<'a>(tokens: &TokenStream<'a>) -> Result<()> {
    inner_display_tokenizer(tokens).map_err(|err| anyhow::anyhow!("in display_tokenizer: {err}"))
}

fn inner_display_tokenizer<'a>(tokens: &TokenStream<'a>) -> Result<()> {
    let mut output_path = config::CONFIG.output_path().join("tokenizer");
    output_path.push("tokens.soulc");

    let mut writer = write_create_file(&output_path)?;
    display_tokens(tokens.clone(), &mut writer)?;
    Ok(())
}

fn display_tokens<'a>(tokens: TokenStream<'a>, writer: &mut impl Writer) -> Result<()> {
    for token in tokens {
        let token = token.map_err(|err| anyhow::Error::msg(format!("{err:?}")))?;
        let span_str = format!("{:?}", token.span);
        let tab = " ".repeat(30 - span_str.len());

        writer.push_fmt(format_args!("Span({span_str}){tab}>> ",))?;
        display_tokenkind(&token.kind, writer)?;
        writer.push_char('\n')?;
    }
    writer.writer_flush()
}

fn display_tokenkind(token: &TokenKind, writer: &mut impl Writer) -> Result<()> {
    match token {
        TokenKind::Literal(literal) => {
            match literal {
                TokenLiteral::String(str) => match str {
                    StringLiteral::Str(str) => writer.push_fmt(format_args!("str({str:?})"))?,
                    StringLiteral::Cstr(str) => writer.push_fmt(format_args!("cstr({str:?})"))?,
                },
                TokenLiteral::Number(num) => match num {
                    Number::Int(n) => writer.push_fmt(format_args!("{n}"))?,
                    Number::Uint(n) => writer.push_fmt(format_args!("{n}"))?,
                    Number::Float(n) => writer.push_fmt(format_args!("{n}"))?,
                },
                TokenLiteral::Char(ch) => writer.push_fmt(format_args!("{ch:?}"))?,
            }
        }
        TokenKind::Keyword(key_word) => {
            writer.push_fmt(format_args!("{}", key_word.as_str()))?
        }
        TokenKind::Symbol(symbol) => writer.push_fmt(format_args!("'{}'", symbol.as_str()))?,
        TokenKind::Types(types) => writer.push_fmt(format_args!("{}", types.as_str()))?,
        TokenKind::Ident(ident) => writer.push_fmt(format_args!("{ident:?}"))?,
        TokenKind::EndLine => _ = writer.push_str("'\\n'")?,
        TokenKind::EndFile => _ = writer.push_str("<end-file>")?,
        TokenKind::StringFormat(tag) => {
            writer.push_fmt(format_args!("fstring_start({})", tag.as_str()))?;
        }
        TokenKind::FStringPart(text) => {
            writer.push_fmt(format_args!("fstring_part({text:?})"))?;
        }
        TokenKind::FStringEnd => {
            writer.push_str("fstring_end()")?;
        }
    };

    Ok(())
}
