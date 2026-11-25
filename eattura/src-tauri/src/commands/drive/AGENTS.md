# Google Drive backup

`oauth.rs` authenticates with PKCE (`drive.file`) over a temporary loopback callback and stores credentials in the OS keyring. `backup.rs` builds and verifies a deterministic structured ZIP (`eattura-backup.zip`) and merges concurrent edits without losing invoices, clients, receipts or payment records. `mod.rs` updates a single Drive file in place via `PATCH` and checks Drive metadata (`modifiedTime`, `version`, `appProperties.snapshotSha256`) every 5 minutes and after batched local saves.
