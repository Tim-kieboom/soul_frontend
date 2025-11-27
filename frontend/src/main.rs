extern crate frontend;

use frontend::{ParseResonse, parse_file};
use models::abstract_syntax_tree::syntax_display::SyntaxDisplay;
use std::{fs::File, io::{BufReader, Write}, process::exit};

fn main() {
    
    const MAIN_FILE: &str = "F:\\Code\\Github\\soul_frontend\\frontend\\soul_src\\main.soul";
    let file = File::open(MAIN_FILE)
        .expect("can not open file");

    let ParseResonse{syntax_tree, errors} = match parse_file(BufReader::new(file)) {
        Ok(val) => val,
        Err(err) => {
            eprintln!("{err}");
            exit(1);
        }
    };

    const SYNTAX_TREE: &str = "F:\\Code\\Github\\soul_frontend\\frontend\\tree.soulc";
    let tree_string = syntax_tree.root.display();
    let mut file = File::create(SYNTAX_TREE)
        .expect("can not open file");

    file.write(tree_string.as_bytes()).unwrap();

    for error in errors {
        eprintln!("{}", error.to_message());
    }


    // let errors = sementic_analyse(&syntax_tree);
    // for error in errors {
    //    eprintln!("{}", error.to_message())
    // }
}
