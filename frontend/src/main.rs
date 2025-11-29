extern crate frontend;
use frontend::{ParseResonse, parse_file, utils::convert_error_message::Level};
use models::{abstract_syntax_tree::{AbstractSyntaxTree, syntax_display::SyntaxDisplay}};
use crate::frontend::utils::convert_error_message::ToMessage;
use std::{fs::File, io::{self, BufReader, Read, Write}, process::exit};

fn main() -> io::Result<()> {
    const MAIN_FILE: &str = "F:\\Code\\Github\\soul_frontend\\frontend\\soul_src\\main.soul";
    const SYNTAX_TREE: &str = "F:\\Code\\Github\\soul_frontend\\frontend\\tree.soulc";
    const RELATIVE_PATH: &str = "main.soul";

    let source_file = get_source_file(MAIN_FILE)?;
    let ParseResonse{syntax_tree, errors} = match parse_file(RELATIVE_PATH, &source_file) {
        Ok(val) => val,
        Err(err) => {
            eprintln!("{err}");
            exit(1);
        }
    };

    print_syntax_tree(&syntax_tree, SYNTAX_TREE);

    for error in errors {
        eprintln!("{}\n", error.to_message(Level::Error, RELATIVE_PATH, &source_file));
    }

    Ok(())
}

fn print_syntax_tree(syntax_tree: &AbstractSyntaxTree, path: &str) {
    let tree_string = syntax_tree.root.display();
    let mut out_file = File::create(path)
        .expect("can not open file");

    out_file.write(tree_string.as_bytes()).unwrap();
}

fn get_source_file(path: &str) -> io::Result<String> {
    let file = File::open(path)
        .expect("can not open file");

    let mut reader = BufReader::new(file);
    let mut source_file = String::new();
    reader.read_to_string(&mut source_file)?;

    Ok(source_file)
}
