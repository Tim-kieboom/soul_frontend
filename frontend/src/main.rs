extern crate frontend;
use frontend::{SementicFault, SementicLevel, parse_file, utils::{char_colors::sementic_level_color, convert_error_message::ToMessage}};
use soul_ast::{
    abstract_syntax_tree::{
        AbstractSyntaxTree,
        syntax_display::{DisplayKind, SyntaxDisplay},
    },
};
use std::{
    fs::File,
    io::{self, BufReader, Read, Write},
    process::exit,
};

fn main() -> io::Result<()> {
    const FATAL_LEVEL: SementicLevel = SementicLevel::Error;

    const RELATIVE_PATH: &str = "main.soul";
    const AST_TREE: &str = "D:/Code/Github/soul_frontend/frontend/AST_parsed.soulc";
    const SEMENTIC_TREE: &str = "D:/Code/Github/soul_frontend/frontend/AST_name_resolved.soulc";
    const MAIN_FILE: &str = "D:/Code/Github/soul_frontend/frontend/soul_src/main.soul";

    let source_file = get_source_file(MAIN_FILE)?;
    let reponse = match parse_file(&source_file) {
        Ok(val) => val,
        Err(err) => {
            eprintln!("{err}");
            exit(1);
        }
    };

    let fatal_count = reponse.get_fatal_count(FATAL_LEVEL);
    for fault in &reponse.sementic_info.faults {
        print_fault(fault, &source_file, RELATIVE_PATH);
    }

    if fatal_count > 0 {
        report_final_fail(fatal_count);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "compilation failed read io::stderr to learn more",
        ));
    }

    print_syntax_tree(&reponse.syntax_tree, AST_TREE, DisplayKind::Parser)?;
    print_syntax_tree(&reponse.syntax_tree, SEMENTIC_TREE, DisplayKind::NameResolver)?;
    Ok(())
}

fn report_final_fail(error_len: usize) {
    use frontend::utils::char_colors::*;
    eprintln!(
        "{RED}code failed:{DEFAULT} code could not compile because of {BLUE}{error_len}{DEFAULT} {}",
        if error_len == 1 { "error" } else { "errors" }
    );
}

fn print_syntax_tree(
    syntax_tree: &AbstractSyntaxTree,
    path: &str,
    kind: DisplayKind,
) -> io::Result<()> {
    let tree_string = syntax_tree.root.display(kind);
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

fn print_fault(fault: &SementicFault, source_file: &str, path: &str) {
    use frontend::utils::char_colors::DEFAULT;
    let level = fault.get_level();
    let color = sementic_level_color(&level);
    eprintln!(
        "{color}{}:{DEFAULT} {}\n", 
        level.as_str(),
        fault.get_soul_error().to_message(
            SementicLevel::Error,
            path,
            source_file
        )
    );
}

