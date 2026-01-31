# Bonsai Trie Visualizer Plan

Goal: A Rust-native visualizer for Madara's Bonsai-backed Merkle-Patricia tries, with tree exploration, path tracing, diffs, and proof views.

Repo: `/Users/mohit/Desktop/karnot/bonsai-trie-visualizer/`

---

Detailed Plan

Tech Stack (Rust-first)
- UI: `egui` + `eframe` (desktop native, immediate-mode)
- DB: `rocksdb` crate (read-only)
- Serialization: `parity-scale-codec`, `bitvec`
- Hashing: `starknet-types-core`, `starknet-crypto`
- CLI: `clap` (db path, block range, trie selector)
- Optional utilities: `thiserror`, `anyhow`, `tracing`, `tracing-subscriber`

Repo / Directory Structure
```
bonsai-trie-visualizer/
├── Cargo.toml
├── README.md
├── PLAN.md
├── src/
│   ├── main.rs                # CLI + app bootstrap
│   ├── app.rs                 # egui app state + routing
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── tree_view.rs
│   │   ├── path_trace.rs
│   │   ├── diff_view.rs
│   │   ├── proof_view.rs
│   │   └── panels.rs          # shared widgets (details, stats, errors)
│   ├── db/
│   │   ├── mod.rs
│   │   ├── rocks.rs           # read-only CF open + stats
│   │   └── cf_map.rs          # CF name registry + helpers
│   ├── bonsai/
│   │   ├── mod.rs
│   │   ├── node.rs            # Binary/Edge decode wrappers
│   │   ├── path.rs            # Felt -> 251-bit path conversion
│   │   ├── trie_reader.rs     # node load + cache
│   │   ├── diff_reader.rs     # trie-log parsing
│   │   └── proof.rs           # proof reconstruction
│   ├── model/
│   │   ├── mod.rs
│   │   ├── trie.rs            # trie type enums + identifiers
│   │   └── block.rs           # block selection model
│   └── util/
│       ├── hex.rs
│       └── errors.rs
└── assets/
    └── screenshots/           # manual screenshots, optional
```

Build/Run Targets
- `cargo run -- --db-path /path/to/madara/db`
- `cargo run -- --db-path ... --block 15420`
- `cargo run -- --db-path ... --diff 15419 15420`

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

Detailed Tasks by Phase

Phase 0: Project Scaffold
- Create `Cargo.toml` with eframe/egui + core deps
- `main.rs` parses CLI + launches eframe
- `app.rs` shows a window with tabs + placeholder panels
- Commit: `phase-0: project scaffold with egui app shell`

Phase 1: DB Connection
- `db/rocks.rs` open RocksDB read-only with all CFs
- Display DB path, CF count, latest block (if derivable), and size stats
- Error panel for open failures (permissions, missing CFs)
- Commit: `phase-1: read-only rocksdb open and stats panel`

Phase 2: Trie Reader Core
- `bonsai/node.rs` decode Binary/Edge nodes (SCALE)
- `bonsai/path.rs` felt -> 251-bit path conversion
- `bonsai/trie_reader.rs` load node by path key (identifier + path bytes)
- Cache nodes and expose APIs for tree traversal
- Commit: `phase-2: bonsai node decode and reader`

Phase 3: Tree View MVP
- `ui/tree_view.rs` render root + expand/collapse
- Node details panel (hash, height, children)
- Trie selector: Contract / Storage / Class
- Commit: `phase-3: tree view and node details`

Phase 4: Path Tracing
- Input felt key, compute path
- Step-by-step traversal with match/mismatch
- Leaf value + computed proof size (node count)
- Commit: `phase-4: key path tracing`

Phase 5: Diff View
- `bonsai/diff_reader.rs` parse trie-log keys
- Show changed leaves between blocks
- Root change summary; warn when logs pruned
- Commit: `phase-5: diff view from trie logs`

Phase 6: Proof Visualizer
- Build merkle proof for key
- Visualize proof nodes and verify hash chain
- Export proof JSON
- Commit: `phase-6: proof viewer and export`

Phase 7: Polish & Performance
- Virtualized tree view, lazy loading
- UI polish: colors, layout, error UX
- Update README with usage + screenshots
- Commit: `phase-7: polish and performance`

---

Feedback Loop (per phase)
1. Implement the phase tasks.
2. Run the app and verify:
   - Basic flows with a local Madara DB (use `/tmp/madara_devnet_poc_v2/` when available)
   - Error states (missing CFs, invalid key)
3. Use `agent-browser` to capture screenshots of the UI.
4. If issues found, fix them and re-run:
   - Build errors: adjust deps or code
   - Runtime errors: add guards, improve error messages
   - UI issues: adjust layout, sizing, or panel logic
5. Commit fixes with `phase-N: fix <short desc>` and push.

---

Commit & Push Policy
- Every phase ends with a commit and push to `mohiiit/bonsai-trie-visualizer`.
- Fixes discovered during feedback loops are committed separately.
- Keep commit messages short and scoped (phase-N).

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
