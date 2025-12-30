mod convert_ast;
use anyhow::Result;

static PATH: &[u8] = include_bytes!("../path.json");

fn main() -> Result<()> {
    let path = from_raw_path(PATH)?;
    run(&path)
}

fn run(path: &str) -> Result<()> {
    convert_ast::run(path)?;
    Ok(())
}

fn from_raw_path(raw_path: &[u8]) -> Result<String> {
    let JsonPath{path} = serde_json::from_slice(raw_path)?;
    Ok(path)
}

#[derive(serde::Deserialize)]
struct JsonPath {path: String}