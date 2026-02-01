use bitvec::order::Msb0;
use bitvec::slice::BitSlice;
use bitvec::vec::BitVec;
use bitvec::view::{AsBits, AsMutBits};
use starknet_types_core::felt::Felt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathBits(pub BitVec<u8, Msb0>);

impl Default for PathBits {
    fn default() -> Self {
        Self(BitVec::with_capacity(251))
    }
}

impl PathBits {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn push(&mut self, bit: bool) {
        self.0.push(bit);
    }

    pub fn extend_from_bitslice(&mut self, bits: &BitSlice<u8, Msb0>) {
        self.0.extend_from_bitslice(bits);
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        if self.0.is_empty() {
            return vec![0u8];
        }
        let len = self.0.len() as u8;
        let mut out = Vec::with_capacity(1 + self.0.as_raw_slice().len());
        out.push(len);
        out.extend_from_slice(self.0.as_raw_slice());
        out
    }

    pub fn from_encoded(bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            return Self::default();
        }
        let len = bytes[0] as usize;
        let mut bits = BitSlice::<u8, Msb0>::from_slice(&bytes[1..]).to_bitvec();
        bits.truncate(len);
        Self(bits)
    }

    pub fn with_bit(&self, bit: bool) -> Self {
        let mut next = self.0.clone();
        next.push(bit);
        Self(next)
    }
}

pub fn felt_to_path(felt: &Felt) -> PathBits {
    let bytes = felt.to_bytes_be();
    let bits: BitVec<u8, Msb0> = bytes.as_bits()[5..].to_owned();
    PathBits(bits)
}

pub fn path_to_felt(path: &PathBits) -> Felt {
    let mut bytes = [0u8; 32];
    bytes
        .as_mut_bits::<Msb0>()[256 - path.len()..]
        .copy_from_bitslice(&path.0);
    Felt::from_bytes_be(&bytes)
}
