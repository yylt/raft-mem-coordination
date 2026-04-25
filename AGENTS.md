# AGENTS.md

## Repo Shape
- Rust crate with one binary entrypoint in `src/main.rs` and library code in `src/lib.rs`.
- Core app wiring lives in `src/app.rs`, config loading in `src/config.rs`, Raft storage in `src/store/`, and HTTP handlers in `src/network/`.
- Integration test harnesses are split between `src/test.rs` and `tests/cluster/`.

## Commands
- `cargo build` builds the server binary.
- `cargo test` runs the unit test plus the cluster integration test.
- `cargo test test_mem_store --lib` runs only the in-memory Raft storage test.
- `cargo test --test cluster -- --nocapture` runs the cluster integration test with logs.
- `./test-cluster.sh` builds, kills any existing `raft-key-value` processes, then drives a 5-node local cluster with `curl`.

## Runtime Notes
- `src/main.rs` loads config from the first CLI arg, defaulting to `config.yaml`.
- `src/config.rs` supports optional `tls_cert_file`, `tls_key_file`, and `basic_auth`; if auth is set, only `/api/*` routes are wrapped, while Raft and coordination endpoints stay open.
- Local cluster tests use fixed loopback ports `21001`-`21005`; avoid running other services there.
- `test-cluster.sh` expects `curl`, and formats output with `jq` when available.

## Implementation Details That Matter
- `create_app` in `src/lib.rs` constructs the Raft node and hard-codes the Raft timing values; do not assume they come from config.
- The state machine is in-memory and snapshotting is serialized through `serde_json` in `src/store/mod.rs`.
- The public API is split into Raft RPCs (`/raft-*`), application API (`/api/write`, `/api/read`, `/api/consistent_read`), and cluster management (`/api/init`, `/api/add-learner`, `/api/change-membership`, `/api/metrics`).
- The repo keeps `Cargo.lock`; do not delete it.
