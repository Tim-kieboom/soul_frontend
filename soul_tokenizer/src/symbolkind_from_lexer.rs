use soul_utils::symbool_kind::SymbolKind;

use crate::lexer::Lexer;

pub trait FromLexer
where
    Self: Sized,
{
    fn from_lexer<'a>(lexer: &mut Lexer<'a>) -> Option<Self>;
}

impl FromLexer for SymbolKind {
    fn from_lexer<'a>(lexer: &mut Lexer<'a>) -> Option<Self> {
        let current = lexer.current_char()?;

        match current {
            '@' => Some(SymbolKind::ConstRef),
            '$' => Some(SymbolKind::Money),
            '+' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    lexer.next_char();
                    Some(SymbolKind::PlusEq)
                } else if peek == Some('+') {
                    lexer.next_char();
                    Some(SymbolKind::DoublePlus)
                } else {
                    Some(SymbolKind::Plus)
                }
            }
            '-' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    lexer.next_char();
                    Some(SymbolKind::MinusEq)
                } else if peek == Some('-') {
                    lexer.next_char();
                    Some(SymbolKind::DoubleMinus)
                } else {
                    Some(SymbolKind::Minus)
                }
            }
            '%' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    lexer.next_char();
                    Some(SymbolKind::ModEq)
                } else {
                    Some(SymbolKind::Mod)
                }
            }
            '*' => {
                let peek = lexer.peek_char();
                if peek == Some('*') {
                    lexer.next_char();
                    Some(SymbolKind::DoubleStar)
                } else if peek == Some('=') {
                    lexer.next_char();
                    Some(SymbolKind::StarEq)
                } else {
                    Some(SymbolKind::Star)
                }
            }
            '/' => {
                let peek = lexer.peek_char();
                if peek == Some('<') {
                    lexer.next_char();
                    Some(SymbolKind::Root)
                } else if peek == Some('=') {
                    lexer.next_char();
                    Some(SymbolKind::SlashEq)
                } else {
                    Some(SymbolKind::Slash)
                }
            }
            '&' => {
                let peek = lexer.peek_char();
                if peek == Some('&') {
                    lexer.next_char();
                    Some(SymbolKind::DoubleAnd)
                } else if peek == Some('=') {
                    lexer.next_char();
                    Some(SymbolKind::AndEq)
                } else {
                    Some(SymbolKind::And)
                }
            }
            '|' => {
                let peek = lexer.peek_char();
                if peek == Some('|') {
                    lexer.next_char();
                    Some(SymbolKind::DoubleOr)
                } else if peek == Some('=') {
                    lexer.next_char();
                    Some(SymbolKind::OrEq)
                } else {
                    Some(SymbolKind::Or)
                }
            }
            '^' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    Some(Self::XorEq)
                } else {
                    Some(SymbolKind::Xor)
                }
            }
            '=' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    lexer.next_char();
                    Some(SymbolKind::Eq)
                } else if peek == Some('>') {
                    lexer.next_char();
                    Some(SymbolKind::LambdaArray)
                } else {
                    Some(SymbolKind::Assign)
                }
            }
            '!' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    lexer.next_char();
                    Some(SymbolKind::NotEq)
                } else {
                    Some(SymbolKind::Not)
                }
            }
            '?' => {
                let peek = lexer.peek_char();
                if peek == Some('?') {
                    lexer.next_char();
                    Some(SymbolKind::DoubleQuestion)
                } else {
                    Some(SymbolKind::Question)
                }
            }
            '<' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    lexer.next_char();
                    Some(SymbolKind::Le)
                } else {
                    Some(SymbolKind::LeftArray)
                }
            }
            '>' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    lexer.next_char();
                    Some(SymbolKind::Ge)
                } else {
                    Some(SymbolKind::RightArray)
                }
            }
            ':' => {
                let peek = lexer.peek_char();
                if peek == Some(':') {
                    lexer.next_char();
                    Some(SymbolKind::DoubleColon)
                } else if peek == Some('=') {
                    lexer.next_char();
                    Some(SymbolKind::ColonAssign)
                } else {
                    Some(SymbolKind::Colon)
                }
            }
            ';' => Some(SymbolKind::SemiColon),
            '.' => {
                let peek = lexer.peek_char();
                if peek == Some('.') {
                    lexer.next_char();
                    Some(SymbolKind::DoubleDot)
                } else {
                    Some(SymbolKind::Dot)
                }
            }
            ',' => Some(SymbolKind::Comma),
            '[' => {
                let peek = lexer.peek_char();
                if peek == Some(']') {
                    lexer.next_char();
                    Some(SymbolKind::Array)
                } else {
                    Some(SymbolKind::SquareOpen)
                }
            }
            ']' => Some(SymbolKind::SquareClose),
            '(' => Some(SymbolKind::RoundOpen),
            ')' => Some(SymbolKind::RoundClose),
            '{' => Some(SymbolKind::CurlyOpen),
            '}' => Some(SymbolKind::CurlyClose),
            _ => None,
        }
    }
}
