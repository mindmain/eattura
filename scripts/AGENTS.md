# Development scripts

- `install.sh` installs the latest verified macOS (`.app.tar.gz`) or Linux (`.AppImage`) release from GitHub Releases, checking `SHA256SUMS` and clearing macOS quarantine attributes.
- `release.mjs` validates release metadata, exports pinned Buf dependencies and stages installers, updater signatures, `latest.json` and checksums. Run `node --test scripts/release.test.mjs` from the root.
- `run-tauri-dev.sh` starts Tauri. The macOS runner uses a matching Keychain signing identity or falls back to ad-hoc signing. Keep the app identifier stable for Keychain access.
