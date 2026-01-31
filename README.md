# Bonsai Trie Visualizer

Rust-native visualizer for Madara's Bonsai-backed Merkle Patricia tries.

## Usage

```bash
cargo run -- --db-path /tmp/madara_devnet_poc_v2/
```

## Notes

- DB must include all Madara column families; the app validates Bonsai columns on open.
- Storage trie requires a contract address (felt) as the identifier.

See `PLAN.md` for the roadmap and workflow.
