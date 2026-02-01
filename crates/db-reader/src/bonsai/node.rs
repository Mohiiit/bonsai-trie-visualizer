use bitvec::order::Msb0;
use bitvec::slice::BitSlice;
use bitvec::vec::BitVec;
use parity_scale_codec::{Decode, Encode, Error, Input, Output};
use starknet_types_core::felt::Felt;

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum Node {
    Binary(BinaryNode),
    Edge(EdgeNode),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum NodeHandle {
    Hash(Felt),
    InMemory(u64),
}

impl NodeHandle {
    pub fn as_hash(self) -> Option<Felt> {
        match self {
            NodeHandle::Hash(felt) => Some(felt),
            NodeHandle::InMemory(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct BinaryNode {
    pub hash: Option<Felt>,
    pub height: u64,
    pub left: NodeHandle,
    pub right: NodeHandle,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct EdgeNode {
    pub hash: Option<Felt>,
    pub height: u64,
    pub path: Path,
    pub child: NodeHandle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path(pub BitVec<u8, Msb0>);

impl Path {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_bits(&self) -> &BitSlice<u8, Msb0> {
        &self.0
    }
}

impl Encode for Path {
    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        let len = self.0.len();
        dest.push_byte(len as u8);
        let mut next_store: u8 = 0;
        let mut pos_in_next_store: u8 = 7;
        for b in self.0.iter() {
            let bit = if *b { 1 } else { 0 };
            next_store |= bit << pos_in_next_store;

            if pos_in_next_store == 0 {
                pos_in_next_store = 8;
                dest.push_byte(next_store);
                next_store = 0;
            }
            pos_in_next_store -= 1;
        }
        if pos_in_next_store < 7 {
            dest.push_byte(next_store);
        }
    }
}

impl Decode for Path {
    fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
        let len: u8 = input.read_byte()?;
        let mut remaining_bits = len as usize;
        let mut current_byte = None;
        let mut bit = 7;
        let mut bits = BitVec::new();
        while remaining_bits != 0 {
            let store = match current_byte {
                Some(store) => store,
                None => {
                    let store = input.read_byte()?;
                    current_byte = Some(store);
                    store
                }
            };

            let res = match (store >> bit) & 1 {
                0 => false,
                1 => true,
                _ => unreachable!("bit must be 0 or 1"),
            };
            bits.push(res);

            remaining_bits -= 1;
            if bit == 0 {
                current_byte = None;
                bit = 8;
            }
            bit -= 1;
        }
        Ok(Self(bits))
    }
}
