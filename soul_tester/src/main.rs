use anyhow::Result;
use std::{fs::OpenOptions, io::Write, path::Path, sync::LazyLock};
use soul_tokenizer::{TokenStream, to_token_stream};
use soul_utils::{ids::IdAlloc, span::ModuleId};

use crate::display::tokenizer::display_tokens;

pub(crate) mod display;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Configs {
    pub main_path: String,
    pub output_path: String,
}

const RAW_CONFIG: &str = include_str!("../config.json");
pub const CONFIG: LazyLock<Configs> = LazyLock::new(|| serde_json::from_str(RAW_CONFIG).expect("should have not parse error") );

fn main() {
    if let Err(err) = frontend() {
        eprintln!("{err}");
    }
}

fn frontend() -> Result<()> {
    
    let file = std::fs::read_to_string(&CONFIG.main_path)?;
    let tokens = to_token_stream(&file, ModuleId::begin())
        .map_err(|err| anyhow::Error::msg(err.to_string()))?;

    display_tokenizer(tokens)?;
    Ok(())
}

fn display_tokenizer<'a>(tokens: TokenStream<'a>) -> Result<()> {
    let output_path = Path::new(&CONFIG.output_path).join("tokenizer\\tokens.soulc");
    std::fs::create_dir_all(&output_path.parent().expect("just joined a parent"))?;
    let mut writer = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&output_path)
        .map_err(|err| anyhow::anyhow!("Failed to create output file({output_path:?}): {}", err))?;

    display_tokens(tokens, &mut writer)?;
    writer.flush()?;
    Ok(())
}
