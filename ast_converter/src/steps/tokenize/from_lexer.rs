use crate::steps::tokenize::tokenizer::Lexer;
use soul_utils::SymboolKind;

pub trait FromLexer
where
    Self: Sized,
{
    fn from_lexer<'a>(lexer: &mut Lexer<'a>) -> Option<Self>;
}

impl FromLexer for SymboolKind {
    fn from_lexer<'a>(lexer: &mut Lexer<'a>) -> Option<Self> {
        let current = lexer.current_char()?;

        match current {
            '@' => Some(SymboolKind::ConstRef),
            '$' => Some(SymboolKind::Money),
            '+' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    lexer.next_char();
                    Some(SymboolKind::PlusEq)
                } else if peek == Some('+') {
                    lexer.next_char();
                    Some(SymboolKind::DoublePlus)
                } else {
                    Some(SymboolKind::Plus)
                }
            }
            '-' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    lexer.next_char();
                    Some(SymboolKind::MinusEq)
                } else if peek == Some('-') {
                    lexer.next_char();
                    Some(SymboolKind::DoubleMinus)
                } else {
                    Some(SymboolKind::Minus)
                }
            }
            '%' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    lexer.next_char();
                    Some(SymboolKind::ModEq)
                } else {
                    Some(SymboolKind::Mod)
                }
            }
            '*' => {
                let peek = lexer.peek_char();
                if peek == Some('*') {
                    lexer.next_char();
                    Some(SymboolKind::DoubleStar)
                } else if peek == Some('=') {
                    lexer.next_char();
                    Some(SymboolKind::StarEq)
                } else {
                    Some(SymboolKind::Star)
                }
            }
            '/' => {
                let peek = lexer.peek_char();
                if peek == Some('<') {
                    lexer.next_char();
                    Some(SymboolKind::Root)
                } else if peek == Some('=') {
                    lexer.next_char();
                    Some(SymboolKind::SlashEq)
                } else {
                    Some(SymboolKind::Slash)
                }
            }
            '&' => {
                let peek = lexer.peek_char();
                if peek == Some('&') {
                    lexer.next_char();
                    Some(SymboolKind::DoubleAnd)
                } else if peek == Some('=') {
                    lexer.next_char();
                    Some(SymboolKind::AndEq)
                } else {
                    Some(SymboolKind::And)
                }
            }
            '|' => {
                let peek = lexer.peek_char();
                if peek == Some('|') {
                    lexer.next_char();
                    Some(SymboolKind::DoubleOr)
                } else if peek == Some('=') {
                    lexer.next_char();
                    Some(SymboolKind::OrEq)
                } else {
                    Some(SymboolKind::Or)
                }
            }
            '^' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    Some(Self::XorEq)
                } else {
                    Some(SymboolKind::Xor)
                }
            }
            '=' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    lexer.next_char();
                    Some(SymboolKind::Eq)
                } else if peek == Some('>') {
                    lexer.next_char();
                    Some(SymboolKind::LambdaArray)
                } else {
                    Some(SymboolKind::Assign)
                }
            }
            '!' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    lexer.next_char();
                    Some(SymboolKind::NotEq)
                } else {
                    Some(SymboolKind::Not)
                }
            }
            '?' => {
                let peek = lexer.peek_char();
                if peek == Some('?') {
                    lexer.next_char();
                    Some(SymboolKind::DoubleQuestion)
                } else {
                    Some(SymboolKind::Question)
                }
            }
            '<' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    lexer.next_char();
                    Some(SymboolKind::Le)
                } else {
                    Some(SymboolKind::LeftArray)
                }
            }
            '>' => {
                let peek = lexer.peek_char();
                if peek == Some('=') {
                    lexer.next_char();
                    Some(SymboolKind::Ge)
                } else {
                    Some(SymboolKind::RightArray)
                }
            }
            ':' => {
                let peek = lexer.peek_char();
                if peek == Some(':') {
                    lexer.next_char();
                    Some(SymboolKind::DoubleColon)
                } else if peek == Some('=') {
                    lexer.next_char();
                    Some(SymboolKind::ColonAssign)
                } else {
                    Some(SymboolKind::Colon)
                }
            }
            ';' => Some(SymboolKind::SemiColon),
            '.' => {
                let peek = lexer.peek_char();
                if peek == Some('.') {
                    lexer.next_char();
                    Some(SymboolKind::DoubleDot)
                } else {
                    Some(SymboolKind::Dot)
                }
            }
            ',' => Some(SymboolKind::Comma),
            '[' => {
                let peek = lexer.peek_char();
                if peek == Some(']') {
                    lexer.next_char();
                    Some(SymboolKind::Array)
                } else {
                    Some(SymboolKind::SquareOpen)
                }
            }
            ']' => Some(SymboolKind::SquareClose),
            '(' => Some(SymboolKind::RoundOpen),
            ')' => Some(SymboolKind::RoundClose),
            '{' => Some(SymboolKind::CurlyOpen),
            '}' => Some(SymboolKind::CurlyClose),
            _ => None,
        }
    }
}
