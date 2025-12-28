mod convert_ast;
use anyhow::Result;

pub const MY_PATH: &str = "D:/Code/Github/soul_frontend/soul_tester";

fn main() {
    
    if let Err(err) = run() {
        eprintln!("{err}");
    }
}

fn run() -> Result<()> {
    convert_ast::run()?;
    Ok(())
}

