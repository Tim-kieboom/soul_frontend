use soul_tokenizer::model::TokenKind;
use soul_utils::{ids::IdAlloc, span::ModuleId};
extern crate soul_tokenizer;

fn main() {
    let mut tokens = match soul_tokenizer::to_token_stream("hello", ModuleId::begin()) {
        Ok(val) => val,
        Err(err) => {
            eprintln!("{err:?}");
            return
        }
    };

    loop {
        let (token, fault) = tokens.consume_advance();
        if let Some(err) = fault {
            eprintln!("{err:?}");
        }

        println!("{}", token.kind.display());
        if token.kind == TokenKind::EndFile {
            break;
        }
    }
}