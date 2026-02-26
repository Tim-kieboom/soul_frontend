use soul_utils::symbool_kind::SymbolKind;

use crate::{TokenKind, lexer::Lexer, token::Number};

fn lexer_to_vec(input: &str) -> Vec<TokenKind> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next_token().expect("lexer error");
        if matches!(token.kind, TokenKind::EndFile) {
            break;
        }
        tokens.push(token.kind);
    }

    tokens
}

#[test]
fn lex_single_identifier() {
    let tokens = lexer_to_vec("hello");

    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0], TokenKind::Ident(ref s) if s == "hello"));
}

#[test]
fn lex_multiple_identifiers_with_whitespace() {
    let tokens = lexer_to_vec("foo bar   baz");

    assert_eq!(tokens.len(), 3);
    assert!(matches!(tokens[0], TokenKind::Ident(ref s) if s == "foo"));
    assert!(matches!(tokens[1], TokenKind::Ident(ref s) if s == "bar"));
    assert!(matches!(tokens[2], TokenKind::Ident(ref s) if s == "baz"));
}

#[test]
fn lex_positive_integer_number() {
    let tokens = lexer_to_vec("123");

    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], TokenKind::Number(Number::Uint(123)));
}

#[test]
fn lex_float_number() {
    let tokens = lexer_to_vec("12.34");

    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], TokenKind::Number(Number::Float(12.34)));
}

#[test]
fn lex_identifier_and_number() {
    let tokens = lexer_to_vec("x = 42");

    let expected = vec![
        TokenKind::Ident("x".to_string()),
        TokenKind::Symbol(SymbolKind::Assign),
        TokenKind::Number(Number::Uint(42)),
    ];

    assert_eq!(tokens, expected);
}

#[test]
fn lex_symbols() {
    let tokens = lexer_to_vec("()+-*/{}[ ][]");

    let expected = [
        TokenKind::Symbol(SymbolKind::RoundOpen),
        TokenKind::Symbol(SymbolKind::RoundClose),
        TokenKind::Symbol(SymbolKind::Plus),
        TokenKind::Symbol(SymbolKind::Minus),
        TokenKind::Symbol(SymbolKind::Star),
        TokenKind::Symbol(SymbolKind::Slash),
        TokenKind::Symbol(SymbolKind::CurlyOpen),
        TokenKind::Symbol(SymbolKind::CurlyClose),
        TokenKind::Symbol(SymbolKind::SquareOpen),
        TokenKind::Symbol(SymbolKind::SquareClose),
        TokenKind::Symbol(SymbolKind::Array),
    ];

    assert_eq!(tokens, expected);
}

#[test]
fn skip_line_comments() {
    let tokens = lexer_to_vec(
        r#"
        foo // this is a comment
        bar
        "#,
    );

    let expected = vec![
        TokenKind::EndLine,
        TokenKind::Ident("foo".to_string()),
        TokenKind::EndLine,
        TokenKind::Ident("bar".to_string()),
        TokenKind::EndLine,
    ];

    assert_eq!(tokens, expected);
}

#[test]
fn span_tracking_advances_lines() {
    let mut lexer = Lexer::new("foo\nbar");

    let foo = lexer.next_token().unwrap();
    let bar = lexer.next_token().unwrap();

    assert_eq!(foo.span.start_line, 1);
    assert_eq!(bar.span.start_line, 2);
}
