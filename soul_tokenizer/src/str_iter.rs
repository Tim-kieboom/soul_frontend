#[derive(Debug, Clone)]
pub(crate) struct StrIter<'a> {
    position: usize,
    next_position: usize,
    source: &'a str,
}
impl<'a> StrIter<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self{next_position: 0, position: 0, source}
    }

    pub(crate) fn position(&self) -> usize {
        self.position
    }

    pub(crate) fn next(&mut self) -> Option<char> {
        if let Some(ch) = self.source[self.next_position..].chars().next() {
            self.position = self.next_position;
            self.next_position += ch.len_utf8();
            Some(ch)
        } else {
            None
        }
    }

    pub(crate) fn peek(&self) -> Option<char> {
        self.source[self.next_position..].chars().next()
    }

    pub(crate) fn slice(&self, range: std::ops::Range<usize>) -> &str {
        &self.source[range]
    }
}
