use starknet_types_core::felt::Felt;
use starknet_types_core::hash::{Pedersen, Poseidon, StarkHash};

use crate::bonsai::node::Node;
use crate::bonsai::path::{path_to_felt, PathBits};
use crate::bonsai::trie_reader::TrieReader;
use bonsai_types::TrieKind;

#[derive(Debug, Clone)]
pub enum ProofNode {
    Binary { left: Felt, right: Felt },
    Edge { child: Felt, path: PathBits },
}

impl ProofNode {
    pub fn hash(&self, kind: TrieKind) -> Felt {
        match kind {
            TrieKind::Contract | TrieKind::Storage => hash_node::<Pedersen>(self),
            TrieKind::Class => hash_node::<Poseidon>(self),
        }
    }
}

pub fn build_proof(reader: &mut TrieReader, key: &PathBits) -> Option<Vec<ProofNode>> {
    let mut path = PathBits::default();
    let mut proof = Vec::new();

    let mut current_node = reader.load_root_node()?;

    loop {
        match &current_node {
            Node::Binary(node) => {
                let left = node.left.as_hash()?;
                let right = node.right.as_hash()?;
                proof.push(ProofNode::Binary { left, right });

                let bit_index = path.len();
                if bit_index >= key.len() {
                    return Some(proof);
                }
                let direction = key.0[bit_index];
                path.push(direction);
            }
            Node::Edge(node) => {
                let child = node.child.as_hash()?;
                let edge_bits = PathBits(node.path.0.clone());
                proof.push(ProofNode::Edge {
                    child,
                    path: edge_bits.clone(),
                });
                path.extend_from_bitslice(&edge_bits.0);
                if path.len() >= key.len() {
                    return Some(proof);
                }
            }
        }

        current_node = reader.load_node_by_path(&path)?;
    }
}

pub fn verify_proof(root: Felt, key: &PathBits, proof: &[ProofNode], kind: TrieKind) -> bool {
    let mut current_hash = root;
    let mut current_path = PathBits::default();

    for node in proof {
        if node.hash(kind) != current_hash {
            return false;
        }
        match node {
            ProofNode::Binary { left, right } => {
                if current_path.len() >= key.len() {
                    return false;
                }
                let direction = key.0[current_path.len()];
                current_path.push(direction);
                current_hash = if direction { *right } else { *left };
            }
            ProofNode::Edge { child, path } => {
                if key.0.get(current_path.len()..(current_path.len() + path.len())) != Some(&path.0) {
                    return false;
                }
                current_path.extend_from_bitslice(&path.0);
                current_hash = *child;
            }
        }
    }
    true
}

fn hash_node<H: StarkHash>(node: &ProofNode) -> Felt {
    match node {
        ProofNode::Binary { left, right } => H::hash(left, right),
        ProofNode::Edge { child, path } => hash_edge::<H>(child, path),
    }
}

fn hash_edge<H: StarkHash>(child_hash: &Felt, path: &PathBits) -> Felt {
    let felt_path = path_to_felt(path);
    let length = Felt::from(path.len() as u64);
    H::hash(child_hash, &felt_path) + length
}
