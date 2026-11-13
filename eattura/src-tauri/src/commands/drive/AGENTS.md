# Google Drive import

`oauth.rs` opens the system browser and handles Picker consent with PKCE and a temporary loopback callback. Keep refresh credentials in the OS keyring. `mod.rs` imports direct-child XML/P7M files through invoice deduplication and tracks versions in `drive-import.json`. Drive currently imports only and ignores SdI receipts. Use local fixtures for tests.
