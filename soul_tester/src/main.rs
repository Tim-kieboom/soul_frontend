use anyhow::Result;
use std::{fs::OpenOptions, io::Write, path::Path};
use soul_tokenizer::{TokenStream, to_token_stream};
use soul_utils::{ids::IdAlloc, span::ModuleId};

use crate::{config::CONFIG, display::tokenizer::display_tokens};

mod display;
mod config;

fn main() {
    match frontend() {
        Ok(()) => println!("success"),
        Err(err) => eprintln!("{err}"),
    }
}

fn frontend() -> Result<()> {
    let file = std::fs::read_to_string(&CONFIG.main_path)?;
    let tokens = to_token_stream(&file, ModuleId::begin())
        .map_err(|err| anyhow::Error::msg(err.message()))?;

    display_tokenizer(tokens)?;
    Ok(())
}

fn display_tokenizer<'a>(tokens: TokenStream<'a>) -> Result<()> {
    let output_path = Path::new(&CONFIG.output_path).join("tokenizer").join("tokens.soulc");
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
