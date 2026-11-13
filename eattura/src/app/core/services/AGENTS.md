# Frontend services

Use `tauri.service.ts` for IPC and guard native features in browser previews. MCP events and Drive imports refresh shared stores. Keep credentials and invoice payloads out of diagnostic logs. Use `AppLogService` directly for logger commands to avoid recursion.
