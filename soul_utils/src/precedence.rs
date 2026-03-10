#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct Precedence(usize);
impl Precedence {
    pub const MIN: Precedence = Precedence(0);
    pub const fn new(num: usize) -> Self {
        Self(num)
    }
    pub const fn as_usize(&self) -> usize {
        self.0
    }
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}