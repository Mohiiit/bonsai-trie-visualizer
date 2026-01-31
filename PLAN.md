# Bonsai Trie Visualizer Plan

Goal: A Rust-native visualizer for Madara's Bonsai-backed Merkle-Patricia tries, with tree exploration, path tracing, diffs, and proof views.

Repo: `/Users/mohit/Desktop/karnot/bonsai-trie-visualizer/`

---

Phases Overview

┌───────┬────────────────────────┬────────────────────────────────────────────────────┬──────────────────────────────────────────────────────────┐
│ Phase │          Goal          │                  Commit Message                    │                     Key Deliverables                     │
├───────┼────────────────────────┼────────────────────────────────────────────────────┼──────────────────────────────────────────────────────────┤
│ 0     │ Project Scaffold       │ phase-0: project scaffold with hello world         │ eframe/egui app shell, CLI args, basic window            │
│ 1     │ DB Connection          │ phase-1: read-only rocksdb and column map           │ Open DB read-only, list CFs, health/stats panel          │
│ 2     │ Trie Reader Core       │ phase-2: bonsai trie reader and node decoding       │ Load nodes from trie column, decode nodes, cache         │
│ 3     │ Tree View MVP          │ phase-3: tree view with expand/collapse             │ Tree rendering, node detail panel, root hash display     │
│ 4     │ Path Tracing           │ phase-4: key path trace and leaf lookup             │ Key input, path steps, leaf value, proof size info       │
│ 5     │ Diff View              │ phase-5: block diff viewer from trie logs           │ Read trie logs, changed leaves view, root change         │
│ 6     │ Proof Visualizer       │ phase-6: merkle proof view and export               │ Proof graph, JSON export, validate proof hash chain      │
│ 7     │ Polish & Performance   │ phase-7: polish, caching, and responsiveness        │ UI polish, large-tree perf, error UX, docs               │
└───────┴────────────────────────┴────────────────────────────────────────────────────┴──────────────────────────────────────────────────────────┘

---

Implementation Notes

- Use all RocksDB column families from Madara; the bonsai CFs are a subset.
- Contract storage trie is keyed per-contract: identifier = contract address bytes.
- Trie log key format: [block_id_be][0x00][trie_key_bytes][key_type][change_type].
- Logs may be pruned by max_saved_trie_logs; diff view should warn if unavailable.
- Path encoding is 251-bit (skip first 5 bits of Felt bytes).
- Contract/class/global root hashing must match Madara (Pedersen/Poseidon rules).

---

Development Workflow (Each Phase)

1. IMPLEMENT → Write code and wire to UI
2. VERIFY   → run app, load sample DB, manual sanity checks
3. FEEDBACK → review UI behavior and edge cases
4. COMMIT   → git commit -m "phase-N: description"
5. ITERATE  → fix issues, then move to next phase

---

References

- Madara integration:
  - `madara/crates/client/db/src/rocksdb/trie.rs`
  - `madara/crates/client/db/src/rocksdb/global_trie/`
- Bonsai trie internals:
  - `bonsai-trie/src/trie/merkle_node.rs`
  - `bonsai-trie/src/trie/path.rs`
  - `bonsai-trie/src/trie/proof.rs`
- Plan doc:
  - `madara2/docs/bonsai-trie-visualizer.md`
