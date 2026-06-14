use std::{iter::Peekable, str::Chars};

#[derive(Debug, Clone)]
pub(crate) struct StrIter<'a> {
    chars: Peekable<Chars<'a>>,
    next_position: usize,
    position: usize,
    source: &'a str,
}
impl<'a> StrIter<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self{
            chars: source.chars().peekable(),
            next_position: 0, 
            position: 0, 
            source,
        }
    }

    pub(crate) fn position(&self) -> usize {
        self.position
    }

    pub(crate) fn next(&mut self) -> Option<char> {
        
        self.position = self.next_position;
        if let Some(ch) = self.chars.next() {
            self.next_position += ch.len_utf8();
            Some(ch)
        } else {
            None
        }
    }

    pub(crate) fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    pub(crate) fn slice(&self, range: std::ops::Range<usize>) -> &str {
        &self.source[range]
    }
}
