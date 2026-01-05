mod convert_ast;
mod convert_hir;
mod paths;
use anyhow::Result;

use crate::paths::Paths;

const MAIN_FILE: &str = "main.soul";
static RAW_PATH_JSON: &[u8] = include_bytes!("../path.json");

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err)
    }
}

fn run() -> Result<()> {
    let path = Paths::new(RAW_PATH_JSON)?;

    convert_ast::run(&path, MAIN_FILE)?;
    convert_hir::run(&path, MAIN_FILE)?;
    Ok(())
}
