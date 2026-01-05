use std::{fs::File, io::Read};

use crate::paths::Paths;
use anyhow::Result;
use ast_converter::AstMetaData;
use soul_ast::{ParseResonse, abstract_syntax_tree::AbstractSyntaxTree};

pub fn run(path: &Paths, file_name: &str) -> Result<()> {
    let ast_path = path.get_ast_incremental_ast(file_name);
    let meta_path = path.get_ast_incremental_ast_meta(file_name);

    let ast_bin = read_binary(ast_path)?;
    let meta_bin = read_binary(meta_path)?;

    let meta: AstMetaData = serde_cbor::from_slice(&meta_bin)?;
    let ast: AbstractSyntaxTree = serde_cbor::from_slice(&ast_bin)?;
    let response = ParseResonse {
        sementic_info: meta,
        syntax_tree: ast,
    };

    let hir = hir_converter::lower_abstract_syntax_tree(&response);
    Ok(())
}

fn read_binary(file: String) -> Result<Vec<u8>> {
    let mut buf = vec![];
    File::open(file)?.read_to_end(&mut buf)?;

    Ok(buf)
}
