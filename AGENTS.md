# Eattura

Local-first Italian electronic invoicing for SdI.

- [Desktop app](eattura/AGENTS.md): Angular UI and native Tauri backend.
- [Shared Rust crate](common/AGENTS.md): SdI XML, validation, PDF, PEC and P7M.
- [HTTP server](server-eattura/AGENTS.md): stateless PEC, SdI and VIES endpoints.
- `proto/`: pinned Protobuf submodule. Edit contracts in their upstream repository.
- [Development scripts](scripts/AGENTS.md) and [GitHub automation](.github/AGENTS.md).

Read the nearest AGENTS.md before editing. Keep affected instructions concise and current. Write repository docs, comments and tests in English, and chat replies in Italian. Preserve unrelated changes.

Run `task install`, `task dev`, `task test` or `task build` from the root. Use English Conventional Commits for requested commits and squash PR titles. Share these instructions through Git and keep personal assistant configuration local. Follow [RELEASING.md](RELEASING.md) for desktop releases and version changes.
