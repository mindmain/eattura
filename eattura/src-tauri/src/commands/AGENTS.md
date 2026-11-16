# Native commands

- `invoices.rs` owns draft revisions, validation, XML/P7M import and fiscal deduplication. MCP edits preserve the parties. Targeted line edits keep other fields intact and check the draft revision within the transaction.
- `pec.rs` persists a pending attempt before SMTP. Resolve it using the matching receipt or manual review before retrying.
- `clients.rs` and `settings.rs` own the directory, SdI recipient backfill (`0000000` default for non-PA while preserving explicit code/PEC configurations) and issuer settings. `payments.rs` and `export.rs` handle reporting.
- [drive](drive/AGENTS.md) imports from Google Drive.

Register command changes in `../lib.rs` and align Angular IPC contracts.
