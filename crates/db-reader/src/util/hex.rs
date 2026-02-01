use starknet_types_core::felt::Felt;
use parity_scale_codec::Decode;

pub fn parse_felt_hex(input: &str) -> Result<Felt, String> {
    let s = input.trim();
    let s = s.strip_prefix("0x").unwrap_or(s);
    let s = if s.is_empty() { "0" } else { s };
    let prefixed = format!("0x{s}");
    Felt::from_hex(&prefixed).map_err(|e| format!("invalid felt: {e}"))
}

pub fn format_felt_short(felt: &Felt) -> String {
    let s = format!("{felt:#x}");
    if s.len() <= 14 {
        return s;
    }
    format!("{}…{}", &s[..10], &s[s.len() - 4..])
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2 + 2);
    s.push_str("0x");
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

pub fn decode_felt_scale(bytes: &[u8]) -> Option<Felt> {
    let mut slice = bytes;
    Felt::decode(&mut slice).ok()
}
