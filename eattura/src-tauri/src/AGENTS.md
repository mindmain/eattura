# Native runtime

`lib.rs` opens the locked local archive, registers IPC commands and starts PEC polling. [commands](commands/AGENTS.md) handles user operations. [mcp](mcp/AGENTS.md) uses the same archive via private IPC. [storage](storage/AGENTS.md) documents persistence. Keep PEC passwords in the OS keyring and diagnostics in `app_log.rs`.
