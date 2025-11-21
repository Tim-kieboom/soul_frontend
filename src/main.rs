extern crate soul_frontend;

use std::{fs::File, io::BufReader};

use soul_frontend::compile_frontend;

fn main() {
    
    const MAIN_FILE: &str = "F:\\Code\\Github\\soul_frontend\\soul_src\\main.soul";
    let file = File::open(MAIN_FILE)
        .expect("can not open file");

    if let Err(err) = compile_frontend(BufReader::new(file)) {
        eprintln!("{}", err.to_message())
    }
}
