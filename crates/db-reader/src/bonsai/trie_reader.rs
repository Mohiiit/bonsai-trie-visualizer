use std::collections::HashMap;

use parity_scale_codec::Decode;
use starknet_types_core::felt::Felt;

use crate::bonsai::node::Node;
use crate::bonsai::path::PathBits;
use crate::db::RocksDb;

#[derive(Clone, Debug)]
pub struct TrieSpec {
    pub identifier: Vec<u8>,
    pub trie_cf: String,
    pub flat_cf: String,
    pub log_cf: String,
}

#[derive(Debug)]
pub struct TrieReader {
    db: RocksDb,
    spec: TrieSpec,
    cache: HashMap<Vec<u8>, Node>,
}

impl TrieReader {
    pub fn new(db: RocksDb, spec: TrieSpec) -> Self {
        Self {
            db,
            spec,
            cache: HashMap::new(),
        }
    }

    pub fn db(&self) -> &RocksDb {
        &self.db
    }

    pub fn spec(&self) -> &TrieSpec {
        &self.spec
    }

    pub fn root_path() -> PathBits {
        PathBits::default()
    }

    pub fn load_root_node(&mut self) -> Option<Node> {
        self.load_node_by_path(&Self::root_path())
    }

    pub fn load_node_by_path(&mut self, path: &PathBits) -> Option<Node> {
        let mut key = self.spec.identifier.clone();
        key.extend_from_slice(&path.to_bytes());

        if let Some(node) = self.cache.get(&key) {
            return Some(node.clone());
        }

        let value = self.db.get_cf(&self.spec.trie_cf, &key).ok()??;
        let node = Node::decode(&mut value.as_slice()).ok()?;
        self.cache.insert(key, node.clone());
        Some(node)
    }

    pub fn load_flat_value(&self, key_bits: &PathBits) -> Option<Felt> {
        let mut key = self.spec.identifier.clone();
        key.extend_from_slice(&key_bits.to_bytes());
        let value = self.db.get_cf(&self.spec.flat_cf, &key).ok()??;
        Felt::decode(&mut value.as_slice()).ok()
    }
}
