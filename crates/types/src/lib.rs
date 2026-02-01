use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TrieKind {
    Contract,
    Storage,
    Class,
}

impl TrieKind {
    pub fn identifier(self) -> &'static [u8] {
        match self {
            TrieKind::Contract => b"0xcontract",
            TrieKind::Class => b"0xclass",
            TrieKind::Storage => b"",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeView {
    pub kind: String,
    pub height: u64,
    pub hash: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
    pub child: Option<String>,
    pub path_len: Option<usize>,
    pub path_hex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootResponse {
    pub path_hex: String,
    pub node: Option<NodeView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResponse {
    pub path_hex: String,
    pub node: Option<NodeView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeafResponse {
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub block: u64,
    pub key_type: String,
    pub change_type: String,
    pub key_len: Option<usize>,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResponse {
    pub entries: Vec<DiffEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofNodeJson {
    pub kind: String,
    pub left: Option<String>,
    pub right: Option<String>,
    pub child: Option<String>,
    pub path_len: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofResponse {
    pub verified: bool,
    pub nodes: Vec<ProofNodeJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfsResponse {
    pub total: usize,
    pub names: Vec<String>,
}
