/// Operator precedence level for parsing expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Precedence(u8);
impl Precedence {
    /// Represents no operator (hard break).
    pub const NONE: Precedence = Precedence(u8::MAX);
    /// The minimum precedence level.
    pub const MIN: Precedence = Precedence(0);

    /// Creates a new precedence level from a number.
    pub const fn new(num: u8) -> Self {
        Self(num)
    }

    /// Returns the next higher precedence level.
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }

    pub const fn is_none(self) -> bool {
        self.0 == u8::MAX
    }
}
