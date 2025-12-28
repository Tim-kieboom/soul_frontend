use anyhow::{Error, Result};
use soul_ast::abstract_syntax_tree::{AbstractSyntaxTree, syntax_display::{DisplayKind, SyntaxDisplay}};
use std::{fs::File, io::{BufReader, Read, Write}, process::exit};

use crate::MY_PATH;
use ast_converter::{ParseResonse, SementicFault, SementicLevel, parse_file, utils::convert_error_message::ToMessage};

const RELATIVE_PATH: &str = "main.soul";
const FATAL_LEVEL: SementicLevel = SementicLevel::Error;

pub fn run() -> Result<()> {
    let ast_tree = format!("{MY_PATH}/AST_parsed.soulc");
    let sementic_tree = format!("{MY_PATH}/AST_name_resolved.soulc");
    let main_file = format!("{MY_PATH}/soul_src/main.soul");

    let output_file = format!("{MY_PATH}/output/main.soulAST");
    let json_output_file = format!("{MY_PATH}/output/main.soulAST.json");

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
    write_to_output(
        &response, 
        &output_file,
        Some(&json_output_file),
    )?;
    println!("write output: {GREEN}successfull!!{DEFAULT}");
    Ok(())
}


fn write_to_output(response: &ParseResonse, bin_path: &str, json_path: Option<&str>) -> Result<()> {
    
    if let Some(path) = json_path {
        let binary = serde_json::to_string_pretty(&response)?;

        let mut out_file = File::create(path)?;
        out_file.write(binary.as_bytes())?;
    }

    let binary = serde_cbor::to_vec(&response)?;
    let mut out_file = File::create(bin_path)?;
    out_file.write(&binary)?;
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