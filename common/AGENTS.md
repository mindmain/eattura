# Shared Rust crate

`build.rs` generates Prost types from the pinned `proto/` submodule. [src](src/AGENTS.md) contains the shared invoice implementation. Keep generated code untouched and the crate independent of UI and persistence.

From the root, run `task proto` and `cargo test --locked -p common`.
