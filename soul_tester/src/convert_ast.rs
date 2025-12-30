use anyhow::{Error, Result};
use soul_ast::{abstract_syntax_tree::{AbstractSyntaxTree, syntax_display::{DisplayKind, SyntaxDisplay}}, sementic_models::sementic_fault::{SementicFault, SementicLevel}};
use std::{fs::File, io::{BufReader, Read, Write}, process::exit};

use crate::MY_PATH;
use ast_converter::{ParseResonse, parse_file, utils::convert_error_message::ToMessage};

const RELATIVE_PATH: &str = "main.soul";
const FATAL_LEVEL: SementicLevel = SementicLevel::Error;

pub fn run() -> Result<()> {
    let ast_tree = format!("{MY_PATH}/output/AST_parsed.soulc");
    let sementic_tree = format!("{MY_PATH}/output/AST_name_resolved.soulc");
    let main_file = format!("{MY_PATH}/soul_src/main.soul");

    let source_file = get_source_file(&main_file)?;
    let response = match parse_file(&source_file) {
        Ok(val) => val,
        Err(err) => {
            eprintln!("{err}");
            exit(1);
        }
    };

    let fatal_count = response.get_fatal_count(FATAL_LEVEL);
    for fault in &response.sementic_info.faults {
        print_fault(fault, &source_file, RELATIVE_PATH);
    }

    if fatal_count > 0 {
        report_final_fail(fatal_count);
        return Err(Error::msg(
            "compilation failed read io::stderr to learn more",
        ));
    }

    print_syntax_tree(&response.syntax_tree, &ast_tree, DisplayKind::Parser)?;
    print_syntax_tree(
        &response.syntax_tree,
        &sementic_tree,
        DisplayKind::NameResolver,
    )?;

    use ast_converter::utils::char_colors::{GREEN, DEFAULT};
    println!("parse: {GREEN}successfull!!{DEFAULT}");   
    write_to_output(&response)?;
    println!("write output: {GREEN}successfull!!{DEFAULT}");
    Ok(())
}


fn write_to_output(response: &ParseResonse) -> Result<()> {
    let ast_out_file = format!("{MY_PATH}/output/main.soulAST");
    let sem_out_file = format!("{MY_PATH}/output/main.soulSEM");
    let ast_json_file = format!("{MY_PATH}/output/main.soulAST.json");
    let sem_json_file = format!("{MY_PATH}/output/main.soulSEM.json");

    let tree = serde_json::to_string_pretty(&response.syntax_tree)?;
    let sementic = serde_json::to_string_pretty(&response.sementic_info)?;

    let mut tree_file = File::create(ast_json_file)?;
    tree_file.write(tree.as_bytes())?;
    let mut tree_file = File::create(sem_json_file)?;
    tree_file.write(sementic.as_bytes())?;

    let tree = serde_cbor::to_vec(&response.syntax_tree)?;
    let sementic = serde_cbor::to_vec(&response.sementic_info)?;
    let mut out_file = File::create(ast_out_file)?;
    out_file.write(&tree)?;
    let mut out_file = File::create(sem_out_file)?;
    out_file.write(&sementic)?;
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