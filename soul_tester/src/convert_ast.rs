use crate::paths::Paths;
use anyhow::{Error, Result};
use ast_converter::{
    ParseResonse, SementicFault, SementicLevel, parse_file, utils::convert_error_message::ToMessage,
};
use soul_ast::abstract_syntax_tree::{
    AbstractSyntaxTree,
    syntax_display::{DisplayKind, SyntaxDisplay},
};
use std::{
    fs::File,
    io::{BufReader, Read, Write},
    process::exit,
};

const FATAL_LEVEL: SementicLevel = SementicLevel::Error;

pub fn run(path: &Paths, file_name: &str) -> Result<()> {
    path.insure_paths_exist()?;
    let source_file = get_source_file(&format!("{}/{file_name}", path.soul_src))?;
    let response = match parse_file(&source_file) {
        Ok(val) => val,
        Err(err) => {
            eprintln!("{err}");
            exit(1);
        }
    };

    let fatal_count = response.get_fatal_count(FATAL_LEVEL);
    for fault in &response.sementic_info.faults {
        print_fault(fault, &source_file, &format!("{file_name}"));
    }

    if fatal_count > 0 {
        report_final_fail(fatal_count);
        return Err(Error::msg(
            "compilation failed read io::stderr to learn more",
        ));
    }

    print_syntax_tree(
        &response.syntax_tree,
        &format!("{}/{file_name}_AST.soulc", path.output_ast),
        DisplayKind::Parser,
    )?;

    print_syntax_tree(
        &response.syntax_tree,
        &format!("{}/{file_name}_AST_resolved.soulc", path.output_ast),
        DisplayKind::NameResolver,
    )?;

    use ast_converter::utils::char_colors::{DEFAULT, GREEN};
    println!("parse: {GREEN}successfull!!{DEFAULT}");
    write_to_output(&response, &path, file_name)?;
    println!("write output: {GREEN}successfull!!{DEFAULT}");
    Ok(())
}

fn write_to_output(response: &ParseResonse, path: &Paths, file_name: &str) -> Result<()> {
    use std::fs::File;

    let ast_path = path.get_ast_incremental_ast(file_name);
    let meta_path = path.get_ast_incremental_ast_meta(file_name);

    let ParseResonse {
        syntax_tree,
        sementic_info,
    } = response;

    let binary = serde_cbor::to_vec(syntax_tree)?;
    File::create(&ast_path)?.write(&binary)?;

    let binary = serde_cbor::to_vec(sementic_info)?;
    File::create(&meta_path)?.write(&binary)?;

    let json = serde_json::to_string_pretty(syntax_tree)?;
    File::create(&format!("{ast_path}.json"))?.write(json.as_bytes())?;

    let json = serde_json::to_string_pretty(sementic_info)?;
    File::create(&format!("{meta_path}.json"))?.write(json.as_bytes())?;

    Ok(())
}

fn report_final_fail(error_len: usize) {
    use ast_converter::utils::char_colors::*;
    eprintln!(
        "{RED}code failed:{DEFAULT} code could not compile because of {BLUE}{error_len}{DEFAULT} {}",
        if error_len == 1 { "error" } else { "errors" }
    );
}

fn print_syntax_tree(
    syntax_tree: &AbstractSyntaxTree,
    path: &str,
    kind: DisplayKind,
) -> Result<()> {
    let tree_string = syntax_tree.root.display(kind);
    let mut out_file = File::create(path)?;

    let _write_amount = out_file.write(tree_string.as_bytes())?;
    Ok(())
}

fn get_source_file(path: &str) -> Result<String> {
    let file = File::open(path)?;

    let mut reader = BufReader::new(file);
    let mut source_file = String::new();
    reader.read_to_string(&mut source_file)?;

    Ok(source_file)
}

fn print_fault(fault: &SementicFault, source_file: &str, path: &str) {
    use ast_converter::utils::char_colors::{DEFAULT, sementic_level_color};

    let level = fault.get_level();
    let color = sementic_level_color(&level);
    eprintln!(
        "{color}{}:{DEFAULT} {}\n",
        level.as_str(),
        fault
            .get_soul_error()
            .to_message(SementicLevel::Error, path, source_file)
    );
}
