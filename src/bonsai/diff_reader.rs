use crate::bonsai::path::PathBits;
use crate::db::RocksDb;
use crate::model::TrieKind;

#[derive(Debug, Clone)]
pub struct TrieLogEntry {
    pub block: u64,
    pub trie_kind: TrieKind,
    pub identifier: Vec<u8>,
    pub key_bits: Option<PathBits>,
    pub key_type: u8,
    pub change_type: u8,
    pub value: Vec<u8>,
}

pub fn read_block_log(
    db: &RocksDb,
    log_cf: &str,
    block: u64,
) -> Vec<TrieLogEntry> {
    let mut prefix = block.to_be_bytes().to_vec();
    prefix.push(0x00);

    let mut entries = Vec::new();
    let iter = match db.iter_cf_from(log_cf, &prefix) {
        Ok(iter) => iter,
        Err(_) => return entries,
    };

    for (key, value) in iter {
        if !key.starts_with(&prefix) {
            break;
        }
        if key.len() < 8 + 1 + 2 {
            continue;
        }
        let key_type = key[key.len() - 2];
        let change_type = key[key.len() - 1];
        let trie_key_bytes = &key[(8 + 1)..(key.len() - 2)];

        let (trie_kind, identifier, key_bits) = parse_trie_key(trie_key_bytes);
        entries.push(TrieLogEntry {
            block,
            trie_kind,
            identifier,
            key_bits,
            key_type,
            change_type,
            value,
        });
    }

    entries
}

fn parse_trie_key(bytes: &[u8]) -> (TrieKind, Vec<u8>, Option<PathBits>) {
    let contract_prefix = TrieKind::Contract.identifier();
    if bytes.starts_with(contract_prefix) {
        let key_bytes = bytes[contract_prefix.len()..].to_vec();
        return (
            TrieKind::Contract,
            contract_prefix.to_vec(),
            Some(PathBits::from_encoded(&key_bytes)),
        );
    }

    let class_prefix = TrieKind::Class.identifier();
    if bytes.starts_with(class_prefix) {
        let key_bytes = bytes[class_prefix.len()..].to_vec();
        return (
            TrieKind::Class,
            class_prefix.to_vec(),
            Some(PathBits::from_encoded(&key_bytes)),
        );
    }

    if bytes.len() >= 32 {
        let identifier = bytes[..32].to_vec();
        let key_bytes = bytes[32..].to_vec();
        return (
            TrieKind::Storage,
            identifier,
            Some(PathBits::from_encoded(&key_bytes)),
        );
    }

    (TrieKind::Contract, bytes.to_vec(), None)
}
