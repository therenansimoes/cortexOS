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
        let hex: String = self.0[..4].iter().map(|b| format!("{:02x}", b)).collect();
        write!(f, "sym:{}", hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_generation() {
        let id1 = NodeId::generate();
        let id2 = NodeId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_node_id_from_bytes() {
        let bytes = [1u8; 16];
        let id = NodeId::from_bytes(bytes);
        assert_eq!(id.as_bytes(), &bytes);
    }

    #[test]
    fn test_node_id_display() {
        let id = NodeId::generate();
        let display = format!("{}", id);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_symbol_id_from_bytes() {
        let data = b"test symbol";
        let id1 = SymbolId::from_bytes(data);
        let id2 = SymbolId::from_bytes(data);
        assert_eq!(id1, id2); // Same input should produce same ID
    }

    #[test]
    fn test_symbol_id_different_inputs() {
        let id1 = SymbolId::from_bytes(b"input1");
        let id2 = SymbolId::from_bytes(b"input2");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_symbol_id_display() {
        let id = SymbolId::from_bytes(b"test");
        let display = format!("{}", id);
        assert!(display.starts_with("sym:"));
    }

    #[test]
    fn test_symbol_id_as_bytes() {
        let data = b"test";
        let id = SymbolId::from_bytes(data);
        let bytes = id.as_bytes();
        assert_eq!(bytes.len(), 16);
    }

    #[test]
    fn test_node_id_serialization() {
        let id = NodeId::generate();
        let serialized = bincode::serialize(&id).unwrap();
        let deserialized: NodeId = bincode::deserialize(&serialized).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_symbol_id_serialization() {
        let id = SymbolId::from_bytes(b"test");
        let serialized = bincode::serialize(&id).unwrap();
        let deserialized: SymbolId = bincode::deserialize(&serialized).unwrap();
        assert_eq!(id, deserialized);
    }
}
