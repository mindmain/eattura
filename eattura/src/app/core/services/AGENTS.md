# Frontend services

Use `tauri.service.ts` for IPC and guard native features in browser previews. MCP events and Drive backup sync events refresh shared stores; `updater.service.ts` checks and pre-downloads signed GitHub updates. Keep credentials and invoice payloads out of diagnostic logs. Use `AppLogService` directly for logger commands to avoid recursion.
