use anyhow::Result;
use soul_tokenizer::{Token, TokenStream};
use soul_utils::{error::SoulResult, sementic_level::SementicFault};

use crate::{ERROR_BACKTRACE, convert_soul_error::ToAnyhow, paths::Paths};

pub fn display_tokens<'a>(
    paths: &Paths,
    source_file: &str,
    token_stream: TokenStream<'a>,
) -> Result<String> {
    fn get_token_len(token: SoulResult<Token>) -> usize {
        token.map(|el| el.kind.display_len()).unwrap_or(0)
    }

    let mut sb = "[\n".to_string();
    let max = token_stream
        .clone()
        .into_iter()
        .map(get_token_len)
        .max()
        .unwrap_or(0);

    let len = max + 4;
    for result in token_stream {
        let token = result
            .map_err(|err| SementicFault::error(err).to_anyhow(&paths.source_file, source_file, ERROR_BACKTRACE))?;

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
