/// Operator precedence level for parsing expressions.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct Precedence(usize);
impl Precedence {
    /// The minimum precedence level.
    pub const MIN: Precedence = Precedence(0);

    /// Creates a new precedence level from a number.
    pub const fn new(num: usize) -> Self {
        Self(num)
    }

    /// Returns the precedence as a `usize`.
    pub const fn as_usize(&self) -> usize {
        self.0
    }

    /// Returns the next higher precedence level.
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}
