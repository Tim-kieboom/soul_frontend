use soul_utils::{
    fault::Fault,
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
    try_lex(input).expect("lexer error")
}

/// Like `lexer_to_vec`, but returns the lex error instead of panicking on it —
/// for tests asserting that malformed input is rejected.
fn try_lex(input: &str) -> Result<Vec<TokenKind>, Fault> {
    let mut lexer = Lexer::new(input, module_id());
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next()?;
        if matches!(token.kind, TokenKind::EndFile) {
            break;
        }
        tokens.push(token.kind);
    }

    Ok(tokens)
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

#[test]
fn lex_number_with_type_suffix() {
    let tokens = lexer_to_vec("1_u8, 0_u32, 200_u8");

    assert_eq!(
        tokens,
        vec![
            TokenKind::Literal(TokenLiteral::Number(Number::Uint(1))),
            TokenKind::Symbol(Symbol::Comma),
            TokenKind::Literal(TokenLiteral::Number(Number::Uint(0))),
            TokenKind::Symbol(Symbol::Comma),
            TokenKind::Literal(TokenLiteral::Number(Number::Uint(200))),
        ]
    );
}

// ----------------------------------------------------------------
//  Malformed input (negative cases)
// ----------------------------------------------------------------

#[test]
fn unterminated_string_literal_is_rejected() {
    let err = try_lex(r#""hello"#).expect_err("unterminated string literal should not lex");
    assert!(
        err.message().contains("does not have an end qoute"),
        "unexpected error message: {}",
        err.message()
    );
}

#[test]
fn unterminated_char_literal_is_rejected() {
    let err = try_lex("'").expect_err("unterminated char literal should not lex");
    assert!(
        err.message().contains("Unclosed char literal"),
        "unexpected error message: {}",
        err.message()
    );
}

#[test]
fn char_literal_missing_closing_quote_is_rejected() {
    let err = try_lex("'a").expect_err("char literal missing closing quote should not lex");
    assert!(
        err.message().contains("char literal should end with"),
        "unexpected error message: {}",
        err.message()
    );
}

#[test]
fn unterminated_fstring_literal_is_rejected() {
    let err = try_lex(r#"f"hello"#).expect_err("unterminated f-string literal should not lex");
    assert!(
        err.message().contains("unclosed format string literal"),
        "unexpected error message: {}",
        err.message()
    );
}

#[test]
fn unknown_character_is_rejected() {
    let err = try_lex("`").expect_err("an unrecognized character should not lex");
    assert!(
        err.message().contains("is unknown"),
        "unexpected error message: {}",
        err.message()
    );
}

#[test]
fn well_formed_string_and_char_literals_are_accepted() {
    // Positive counterpart to the unterminated-literal tests above.
    let tokens = lexer_to_vec(r#""hello" 'a'"#);
    assert_eq!(
        tokens,
        vec![
            TokenKind::Literal(TokenLiteral::String(
                soul_utils::literal::StringLiteral::Str("hello".to_string())
            )),
            TokenKind::Literal(TokenLiteral::Char('a')),
        ]
    );
}
