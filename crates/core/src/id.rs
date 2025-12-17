use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(Uuid);

impl NodeId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId([u8; 16]);

impl SymbolId {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut id = [0u8; 16];
        let hash = blake3::hash(bytes);
        id.copy_from_slice(&hash.as_bytes()[..16]);
        Self(id)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl std::fmt::Display for SymbolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sym:{}", hex::encode(&self.0[..4]))
    }
}
