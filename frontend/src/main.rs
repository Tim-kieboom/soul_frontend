extern crate frontend;

use frontend::{ParseResonse, parse_file};
use std::{fs::File, io::BufReader, process::exit};

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

    println!("{:#?}", syntax_tree);

    for error in errors {
        eprintln!("{}", error.to_message());
    }


    // let errors = sementic_analyse(&syntax_tree);
    // for error in errors {
    //    eprintln!("{}", error.to_message())
    // }
}
