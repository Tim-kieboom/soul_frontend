#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NodeId(u32);
impl NodeId {
    pub fn display(&self) -> String {
        format!("{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ScopeId {
    index: usize,
    at_len: usize,
}
