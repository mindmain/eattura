# Local MCP

`bridge.rs` exposes stdio to local clients and forwards only allowlisted tools to the open desktop. `ipc.rs` authenticates private OS IPC. Keep tokens out of client configs and UI responses.

`tools.rs` reads the archive and creates, updates or validates drafts. Prefer `update_draft_lines` for descriptions and dates. Require the current revision, preserve draft parties and emit the saved event. Keep status changes and sending in the desktop. `clients.rs` configures local clients with private backups. Test with `cargo test --locked -p eattura mcp::` and temporary archives/configs.
