extern crate frontend;
use crate::frontend::utils::convert_error_message::ToMessage;
use frontend::{ParseResonse, SementicLevel, parse_file, sementic_analyse};
use models::{abstract_syntax_tree::{AbstractSyntaxTree, syntax_display::SyntaxDisplay}, error::SoulError};
use std::{
    fs::File,
    io::{self, BufReader, Read, Write},
    process::exit,
};

fn main() -> io::Result<()> {
    const RELATIVE_PATH: &str = "main.soul";
    const AST_TREE: &str = "F:/Code/Github/soul_frontend/frontend/AST_tree.soulc";
    const SEMENTIC_TREE: &str = "F:/Code/Github/soul_frontend/frontend/sementic_tree.soulc";
    const MAIN_FILE: &str = "F:/Code/Github/soul_frontend/frontend/soul_src/main.soul";

    let source_file = get_source_file(MAIN_FILE)?;

    let ParseResonse {
        mut syntax_tree,
        errors,
    } = match parse_file(&source_file) {
        Ok(val) => val,
        Err(err) => {
            eprintln!("{err}");
            exit(1);
        }
    };

    
    if !errors.is_empty() {
        report_parse_fail(errors, RELATIVE_PATH, &source_file);
        return Err(fail_err())
    }

    print_syntax_tree(&syntax_tree, AST_TREE)?;

    let mut error_count = 0;
    let faults = sementic_analyse(&mut syntax_tree);
    for fault in faults {
        let level = fault.get_level();
        if level == SementicLevel::Error {
            error_count += 1;
        }
        eprintln!(
            "{}\n",
            fault.consume_soul_error()
                .to_message(SementicLevel::Error, RELATIVE_PATH, &source_file)
        );
    }

    if error_count > 0 {
        report_final_fail(error_count);
        return Err(fail_err())
    }

    print_syntax_tree(&syntax_tree, SEMENTIC_TREE)?;
    Ok(())
}

fn report_parse_fail(errors: Vec<SoulError>, relative_path: &str, source_file: &str) {
    use frontend::utils::char_colors::*;
    let error_len = errors.len();
    for error in errors {
        eprintln!(
            "{}\n",
            error.to_message(SementicLevel::Error, relative_path, source_file)
        );
    }
    eprintln!(
        "{RED}code failed:{DEFAULT} code could not parse because of {BLUE}{error_len}{DEFAULT} {}",
        if error_len == 1 { "error" } else { "errors" }
    );
}

fn report_final_fail(error_len: usize) {
    use frontend::utils::char_colors::*;
    eprintln!(
        "{RED}code failed:{DEFAULT} code could not compile because of {BLUE}{error_len}{DEFAULT} {}",
        if error_len == 1 { "error" } else { "errors" }
    );
}

fn fail_err() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "compilation failed read io::stderr to learn more",
    )
}

fn print_syntax_tree(syntax_tree: &AbstractSyntaxTree, path: &str) -> io::Result<()> {
    let tree_string = syntax_tree.root.display();
    let mut out_file = File::create(path)?;

    let _write_amount = out_file.write(tree_string.as_bytes())?;
    Ok(())
}

fn get_source_file(path: &str) -> io::Result<String> {
    let file = File::open(path)?;

    let mut reader = BufReader::new(file);
    let mut source_file = String::new();
    reader.read_to_string(&mut source_file)?;

    Ok(source_file)
}
