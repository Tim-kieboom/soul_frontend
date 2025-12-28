use soul_ast::symbool_kind::SymboolKind;

use crate::steps::tokenize::{
    Request,
    token_stream::{Token, TokenKind},
    tokenizer::tokenize,
};

fn tokenize_source(source: &str) -> Vec<Token> {
    let request = Request { source };
    tokenize(request)
        .token_stream
        .to_vec()
        .expect("To vec failed")
}

fn to_token_kinds(tokens: Vec<Token>) -> Vec<TokenKind> {
    tokens.iter().map(|el| el.kind.clone()).collect::<Vec<_>>()
}

#[test]
fn test_sum_function_tokens() {
    let source = r#"
        //comment
        sum(/*comment*/one: i32, two: i32) i32 { // comment 
            return one/*comment*/ + two 
            /* comment
            comment
            "string"
            comment
            */
        }
    "#;

    let tokens = tokenize_source(source);

    let expected_kinds = [
        TokenKind::EndLine,
        TokenKind::EndLine,
        TokenKind::Ident("sum".into()),
        TokenKind::Symbool(SymboolKind::RoundOpen),
        TokenKind::Ident("one".into()),
        TokenKind::Symbool(SymboolKind::Colon),
        TokenKind::Ident("i32".into()),
        TokenKind::Symbool(SymboolKind::Comma),
        TokenKind::Ident("two".into()),
        TokenKind::Symbool(SymboolKind::Colon),
        TokenKind::Ident("i32".into()),
        TokenKind::Symbool(SymboolKind::RoundClose),
        TokenKind::Ident("i32".into()),
        TokenKind::Symbool(SymboolKind::CurlyOpen),
        TokenKind::EndLine,
        TokenKind::Ident("return".into()),
        TokenKind::Ident("one".into()),
        TokenKind::Symbool(SymboolKind::Plus),
        TokenKind::Ident("two".into()),
        TokenKind::EndLine,
        TokenKind::EndLine,
        TokenKind::Symbool(SymboolKind::CurlyClose),
        TokenKind::EndLine,
    ];

    assert_eq!(
        tokens.len(),
        expected_kinds.len(),
        "{:#?}",
        to_token_kinds(tokens)
    );

    for (i, Token { kind, .. }) in tokens.into_iter().enumerate() {
        assert_eq!(kind, expected_kinds[i])
    }
}
