use anyhow::Result;
use soul_tokenizer::TokenStream;
use soul_utils::sementic_level::SementicFault;

use crate::{convert_soul_error::ToAnyhow, paths::Paths};

pub fn display_tokens<'a>(
    paths: &Paths,
    source_file: &str,
    token_stream: TokenStream<'a>,
) -> Result<String> {
    let mut sb = "[\n".to_string();

    let max = token_stream
        .clone()
        .into_iter()
        .map(|result| result.map(|el| el.kind.display_len()).unwrap_or(0))
        .max()
        .unwrap_or(0);

    let len = max + 4;
    for result in token_stream {
        let token = result
            .map_err(|err| SementicFault::error(err).to_anyhow(&paths.source_file, source_file))?;

        sb.push('\t');
        let kind_len = token.kind.inner_display(&mut sb)?;
        for _ in 0..len.saturating_sub(kind_len) {
            sb.push_str(" ");
        }
        sb.push_str(">> Span(");
        token.span.inner_display(&mut sb);
        sb.push_str("),\n");
    }

    sb.push(']');
    Ok(sb)
}
