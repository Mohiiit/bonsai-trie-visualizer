#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TrieKind {
    Contract,
    Storage,
    Class,
}

impl TrieKind {
    pub fn label(self) -> &'static str {
        match self {
            TrieKind::Contract => "Contract",
            TrieKind::Storage => "Storage",
            TrieKind::Class => "Class",
        }
    }

    pub fn identifier(self) -> &'static [u8] {
        match self {
            TrieKind::Contract => b"0xcontract",
            TrieKind::Class => b"0xclass",
            TrieKind::Storage => b"",
        }
    }
}
