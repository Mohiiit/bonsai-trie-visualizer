pub mod bonsai;
pub mod db;
pub mod util;

use bonsai::diff_reader::read_block_log;
use bonsai::node::Node;
use bonsai::path::{felt_to_path, PathBits};
use bonsai::proof::{build_proof, verify_proof, ProofNode};
use bonsai::trie_reader::{TrieReader, TrieSpec};
use db::cf_map;
use db::RocksDb;
use bonsai_types::{CfsResponse, DiffEntry, DiffResponse, LeafResponse, NodeResponse, NodeView, ProofNodeJson, ProofResponse, RootResponse, TrieKind};
use util::hex::{bytes_to_hex, decode_felt_scale, format_felt_short, parse_felt_hex};

pub fn open_db(path: &str) -> Result<RocksDb, String> {
    RocksDb::open_read_only(path).map_err(|e| e.to_string())
}

pub fn list_cfs(db: &RocksDb) -> CfsResponse {
    CfsResponse {
        total: db.cf_names().len(),
        names: db.cf_names().to_vec(),
    }
}

pub fn root_node(db: &RocksDb, trie: TrieKind, identifier: Option<String>) -> RootResponse {
    let spec = match build_spec(trie, identifier) {
        Ok(spec) => spec,
        Err(_err) => return RootResponse { path_hex: "0x00".to_string(), node: None },
    };
    let mut reader = TrieReader::new(db.clone(), spec);
    let root_path = PathBits::default();
    let node = reader.load_root_node().map(node_to_view);
    RootResponse {
        path_hex: bytes_to_hex(&root_path.to_bytes()),
        node,
    }
}

pub fn load_node(db: &RocksDb, trie: TrieKind, identifier: Option<String>, path_hex: &str) -> NodeResponse {
    let spec = match build_spec(trie, identifier) {
        Ok(spec) => spec,
        Err(_) => return NodeResponse { path_hex: path_hex.to_string(), node: None },
    };
    let path_bytes = match hex_to_bytes(path_hex) {
        Some(b) => b,
        None => return NodeResponse { path_hex: path_hex.to_string(), node: None },
    };
    let path = PathBits::from_encoded(&path_bytes);
    let mut reader = TrieReader::new(db.clone(), spec);
    let node = reader.load_node_by_path(&path).map(node_to_view);
    NodeResponse { path_hex: path_hex.to_string(), node }
}

pub fn leaf_value(db: &RocksDb, trie: TrieKind, identifier: Option<String>, key_hex: &str) -> LeafResponse {
    let spec = match build_spec(trie, identifier) {
        Ok(spec) => spec,
        Err(_) => return LeafResponse { key: key_hex.to_string(), value: None },
    };
    let felt = match parse_felt_hex(key_hex) {
        Ok(f) => f,
        Err(_) => return LeafResponse { key: key_hex.to_string(), value: None },
    };
    let key_path = felt_to_path(&felt);
    let reader = TrieReader::new(db.clone(), spec);
    let value = reader.load_flat_value(&key_path).map(|v| format!("{v:#x}"));
    LeafResponse { key: key_hex.to_string(), value }
}

pub fn diff_for_block(db: &RocksDb, trie: TrieKind, block: u64) -> DiffResponse {
    let log_cf = match trie {
        TrieKind::Contract => cf_map::BONSAI_CONTRACT_LOG,
        TrieKind::Storage => cf_map::BONSAI_CONTRACT_STORAGE_LOG,
        TrieKind::Class => cf_map::BONSAI_CLASS_LOG,
    };
    let entries = read_block_log(db, log_cf, block)
        .into_iter()
        .map(|entry| {
            let change = match entry.change_type { 0 => "new", 1 => "old", _ => "unknown" };
            let key_type = match entry.key_type { 0 => "trie", 1 => "flat", _ => "unknown" };
            let value_display = decode_felt_scale(&entry.value)
                .map(|felt| format_felt_short(&felt))
                .unwrap_or_else(|| bytes_to_hex(&entry.value));
            DiffEntry {
                block: entry.block,
                key_type: key_type.to_string(),
                change_type: change.to_string(),
                key_len: entry.key_bits.as_ref().map(|k| k.len()),
                value: value_display,
            }
        })
        .collect();
    DiffResponse { entries }
}

pub fn proof_for_key(db: &RocksDb, trie: TrieKind, identifier: Option<String>, key_hex: &str) -> ProofResponse {
    let spec = match build_spec(trie, identifier) {
        Ok(spec) => spec,
        Err(_) => return ProofResponse { verified: false, nodes: Vec::new() },
    };
    let felt = match parse_felt_hex(key_hex) {
        Ok(f) => f,
        Err(_) => return ProofResponse { verified: false, nodes: Vec::new() },
    };
    let key_path = felt_to_path(&felt);
    let mut reader = TrieReader::new(db.clone(), spec);
    let root_node = match reader.load_root_node() {
        Some(n) => n,
        None => return ProofResponse { verified: false, nodes: Vec::new() },
    };
    let root_hash = match &root_node {
        Node::Binary(bin) => bin.hash,
        Node::Edge(edge) => edge.hash,
    };
    let Some(root_hash) = root_hash else {
        return ProofResponse { verified: false, nodes: Vec::new() };
    };
    let proof = match build_proof(&mut reader, &key_path) {
        Some(p) => p,
        None => return ProofResponse { verified: false, nodes: Vec::new() },
    };
    let verified = verify_proof(root_hash, &key_path, &proof, trie);
    let nodes = proof
        .iter()
        .map(|node| match node {
            ProofNode::Binary { left, right } => ProofNodeJson {
                kind: "binary".to_string(),
                left: Some(format!("{left:#x}")),
                right: Some(format!("{right:#x}")),
                child: None,
                path_len: None,
            },
            ProofNode::Edge { child, path } => ProofNodeJson {
                kind: "edge".to_string(),
                left: None,
                right: None,
                child: Some(format!("{child:#x}")),
                path_len: Some(path.len()),
            },
        })
        .collect();
    ProofResponse { verified, nodes }
}

fn build_spec(trie: TrieKind, identifier: Option<String>) -> Result<TrieSpec, String> {
    let identifier = match trie {
        TrieKind::Contract => trie.identifier().to_vec(),
        TrieKind::Class => trie.identifier().to_vec(),
        TrieKind::Storage => {
            let felt = identifier.ok_or_else(|| "missing identifier".to_string())?;
            let felt = parse_felt_hex(&felt).map_err(|e| e.to_string())?;
            felt.to_bytes_be().to_vec()
        }
    };

    let (trie_cf, flat_cf, log_cf) = match trie {
        TrieKind::Contract => (
            cf_map::BONSAI_CONTRACT_TRIE,
            cf_map::BONSAI_CONTRACT_FLAT,
            cf_map::BONSAI_CONTRACT_LOG,
        ),
        TrieKind::Storage => (
            cf_map::BONSAI_CONTRACT_STORAGE_TRIE,
            cf_map::BONSAI_CONTRACT_STORAGE_FLAT,
            cf_map::BONSAI_CONTRACT_STORAGE_LOG,
        ),
        TrieKind::Class => (
            cf_map::BONSAI_CLASS_TRIE,
            cf_map::BONSAI_CLASS_FLAT,
            cf_map::BONSAI_CLASS_LOG,
        ),
    };

    Ok(TrieSpec {
        identifier,
        trie_cf: trie_cf.to_string(),
        flat_cf: flat_cf.to_string(),
        log_cf: log_cf.to_string(),
    })
}

fn node_to_view(node: Node) -> NodeView {
    match node {
        Node::Binary(binary) => NodeView {
            kind: "binary".to_string(),
            height: binary.height,
            hash: binary.hash.map(|h| format!("{h:#x}")),
            left: binary.left.as_hash().map(|h| format!("{h:#x}")),
            right: binary.right.as_hash().map(|h| format!("{h:#x}")),
            child: None,
            path_len: None,
            path_hex: None,
        },
        Node::Edge(edge) => NodeView {
            kind: "edge".to_string(),
            height: edge.height,
            hash: edge.hash.map(|h| format!("{h:#x}")),
            left: None,
            right: None,
            child: edge.child.as_hash().map(|h| format!("{h:#x}")),
            path_len: Some(edge.path.len()),
            path_hex: Some(bytes_to_hex(&PathBits(edge.path.0).to_bytes())),
        },
    }
}

fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    let s = hex.trim().strip_prefix("0x").unwrap_or(hex.trim());
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16).ok()?;
        out.push(byte);
    }
    Some(out)
}
