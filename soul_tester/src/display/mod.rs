use anyhow::Result;
use std::io::Write;
use soul_utils::span::Span;

pub(crate) mod tokenizer;

pub(crate) fn display_span(span: Span, writer: &mut impl Write) -> Result<()> {
    let Span { module:_, start, end } = span;
    writer.write_fmt(format_args!("{}:{}", start.line, start.offset))?;
    if span.is_single() {
        return Ok(())
    } 

    writer.write_fmt(format_args!("-{}:{}", end.line, end.offset))?;
    Ok(())
}