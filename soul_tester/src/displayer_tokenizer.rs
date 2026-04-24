use std::path::Path;

use anyhow::Result;
use soul_tokenizer::{Token, TokenStream};
use soul_utils::{error::SoulResult, sementic_level::SementicFault};

use crate::{MESSAGE_CONFIG, convert_soul_error::ToAnyhow, paths::Paths};

pub fn display_tokens<'a>(
    paths: &Paths,
    source_file: &str,
    token_stream: TokenStream<'a>,
) -> Result<String> {
    fn get_token_len(token: SoulResult<Token>) -> usize {
        token.map(|el| el.kind.display_len()).unwrap_or(0)
    }

    let mut sb = "[\n".to_string();
    let max = token_stream.clone().map(get_token_len).max().unwrap_or(0);

    let manifest = Path::new(&paths.project);
    let len = max + 4;
    for result in token_stream {
        let token = match result {
            Ok(val) => val,
            Err(err) => {
                return Err(SementicFault::error(err).to_anyhow(
                    &Paths::to_entry_file_path(&manifest)?.path,
                    source_file,
                    MESSAGE_CONFIG,
                ));
            }
        };

        sb.push('\t');
        let kind_len = token.kind.inner_display(&mut sb)?;
        for _ in 0..len.saturating_sub(kind_len) {
            sb.push(' ');
        }
        sb.push_str(">> Span(");
        token.span.inner_display(&mut sb);
        sb.push_str("),\n");
    }

    sb.push(']');
    Ok(sb)
}
