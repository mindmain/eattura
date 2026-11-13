# Workflows

- `release-please.yml` creates the desktop version PR, stable tag and draft release.
- `desktop-release.yml` validates the tag and builds on native macOS, Linux and Windows runners. Publish only after all installer checks pass.

Preserve pinned tools and actions, read-only build permissions, `CI=true` for DMG builds and `APPIMAGE_EXTRACT_AND_RUN=1` on Linux. Check changes with `actionlint` and `node --test scripts/release.test.mjs` from the root.
