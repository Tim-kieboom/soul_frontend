use soul_utils::{
    ids::IdAlloc,
    literal::{Number, TokenLiteral},
    soul_names::Symbol,
    span::ModuleId,
};

use crate::{TokenKind, lexer::Lexer, model::StringFormatTag, model::keyword::KeyWord};

fn module_id() -> ModuleId {
    ModuleId::error()
}

fn lexer_to_vec(input: &str) -> Vec<TokenKind> {
    let mut lexer = Lexer::new(input, module_id());
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next().expect("lexer error");
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
    assert!(
        matches!(tokens[0], TokenKind::Ident(ref s) if s == "hello"),
        "`{}` should be `hello`",
        tokens[0].display()
    );
}

#[test]
fn lex_multiple_identifiers_with_whitespace() {
    let tokens = lexer_to_vec("foo bar   baz");

    assert_eq!(tokens.len(), 3);
    assert!(
        matches!(tokens[0], TokenKind::Ident(ref s) if s == "foo"),
        "`{}` should be `foo`",
        tokens[0].display()
    );
    assert!(
        matches!(tokens[1], TokenKind::Ident(ref s) if s == "bar"),
        "`{}` should be `bar`",
        tokens[1].display()
    );
    assert!(
        matches!(tokens[2], TokenKind::Ident(ref s) if s == "baz"),
        "`{}` should be `baz`",
        tokens[2].display()
    );
}

#[test]
fn lex_positive_integer_number() {
    let tokens = lexer_to_vec("123");

    assert_eq!(tokens.len(), 1);
    assert_eq!(
        tokens[0],
        TokenKind::Literal(TokenLiteral::Number(Number::Uint(123)))
    );
}

#[test]
fn lex_float_number() {
    let tokens = lexer_to_vec("12.34");

    assert_eq!(tokens.len(), 1);
    assert_eq!(
        tokens[0],
        TokenKind::Literal(TokenLiteral::Number(Number::Float(12.34)))
    );
}

#[test]
fn lex_identifier_and_number() {
    let tokens = lexer_to_vec("x = 42");

    let expected = vec![
        TokenKind::Ident("x".to_string()),
        TokenKind::Symbol(Symbol::Assign),
        TokenKind::Literal(TokenLiteral::Number(Number::Uint(42))),
    ];

    assert_eq!(tokens, expected);
}

#[test]
fn lex_symbols() {
    let tokens = lexer_to_vec("()+-*/{}[ ][]");

    let expected = [
        TokenKind::Symbol(Symbol::RoundOpen),
        TokenKind::Symbol(Symbol::RoundClose),
        TokenKind::Symbol(Symbol::Plus),
        TokenKind::Symbol(Symbol::Minus),
        TokenKind::Symbol(Symbol::Star),
        TokenKind::Symbol(Symbol::Slash),
        TokenKind::Symbol(Symbol::CurlyOpen),
        TokenKind::Symbol(Symbol::CurlyClose),
        TokenKind::Symbol(Symbol::SquareOpen),
        TokenKind::Symbol(Symbol::SquareClose),
        TokenKind::Symbol(Symbol::Array),
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
    let mut lexer = Lexer::new("foo\nbar", module_id());

    let foo = lexer.next().unwrap();
    let bar = lexer.next().unwrap();

    assert_eq!(foo.span.start.line, 1);
    assert_eq!(bar.span.start.line, 2);
}

// ----------------------------------------------------------------
//  F-string / format string tokenization
// ----------------------------------------------------------------

#[test]
fn fstring_no_expressions() {
    let tokens = lexer_to_vec(r#"f"hello world""#);

    let expected = vec![
        TokenKind::StringFormat(StringFormatTag::F),
        TokenKind::FStringPart("hello world".to_string()),
        TokenKind::FStringEnd,
    ];

    assert_eq!(tokens, expected);
}

#[test]
fn fstring_empty() {
    let tokens = lexer_to_vec(r#"f"""#);

    let expected = vec![
        TokenKind::StringFormat(StringFormatTag::F),
        TokenKind::FStringEnd,
    ];

    assert_eq!(tokens, expected);
}

#[test]
fn fstring_escaped_braces() {
    let tokens = lexer_to_vec(r#"f"{{literal}}""#);

    let expected = vec![
        TokenKind::StringFormat(StringFormatTag::F),
        TokenKind::FStringPart("{literal}".to_string()),
        TokenKind::FStringEnd,
    ];

    assert_eq!(tokens, expected);
}

#[test]
fn fstring_escaped_braces_mixed() {
    let tokens = lexer_to_vec(r#"f"before {{ after""#);

    let expected = vec![
        TokenKind::StringFormat(StringFormatTag::F),
        TokenKind::FStringPart("before { after".to_string()),
        TokenKind::FStringEnd,
    ];

    assert_eq!(tokens, expected);
}

#[test]
fn fstring_fstr_tag_no_expr() {
    let tokens = lexer_to_vec(r#"fstr"hello world""#);

    let expected = vec![
        TokenKind::StringFormat(StringFormatTag::Fstr),
        TokenKind::FStringPart("hello world".to_string()),
        TokenKind::FStringEnd,
    ];

    assert_eq!(tokens, expected);
}

// ----------------------------------------------------------------
//  New keywords (soul-lang.md): union / async / task / spawn / limit / intrinsic
// ----------------------------------------------------------------
#[test]
fn lex_new_keywords() {
    let tokens = lexer_to_vec("union async task spawn limit intrinsic");

    let expected = vec![
        TokenKind::Keyword(KeyWord::Union),
        TokenKind::Keyword(KeyWord::Async),
        TokenKind::Keyword(KeyWord::Task),
        TokenKind::Keyword(KeyWord::Spawn),
        TokenKind::Keyword(KeyWord::Limit),
        TokenKind::Keyword(KeyWord::Intrinsic),
    ];

    assert_eq!(tokens, expected);
}

#[test]
fn lex_capitalized_union_stays_identifier() {
    let tokens = lexer_to_vec("Union");

    let expected = vec![TokenKind::Ident("Union".to_string())];
    assert_eq!(tokens, expected);
}

#[test]
fn lex_right_arrow_symbol() {
    let tokens = lexer_to_vec("a->b");

    let expected = vec![
        TokenKind::Ident("a".to_string()),
        TokenKind::Symbol(Symbol::RightArrow),
        TokenKind::Ident("b".to_string()),
    ];

    assert_eq!(tokens, expected);
}

#[test]
fn lex_star_star_are_separate_symbols() {
    let tokens = lexer_to_vec("2 ** 3");

    let expected = vec![
        TokenKind::Literal(TokenLiteral::Number(Number::Uint(2))),
        TokenKind::Symbol(Symbol::Star),
        TokenKind::Symbol(Symbol::Star),
        TokenKind::Literal(TokenLiteral::Number(Number::Uint(3))),
    ];

    assert_eq!(tokens, expected);
}
