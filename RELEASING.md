# Desktop app releases

Only the desktop app is released. Server deployment, Docker publishing and
npm/crates publishing are disabled. Desktop releases publish installers, signed
Tauri v2 updater artifacts and `latest.json` on GitHub Releases. The first app
release is **0.1.0**, with the canonical Git tag **v0.1.0**.

## Repository setup

Enable GitHub Actions and allow Actions to create pull requests under
**Settings → Actions → General → Workflow permissions**. Workflows request
their own minimum token permissions; no personal access token is required.
Add `TAURI_SIGNING_PRIVATE_KEY` (and optional `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`)
under **Settings → Secrets and variables → Actions** matching the public key in
`eattura/src-tauri/tauri.conf.json`. Keep the GitHub repository public so desktop
clients can fetch `latest.json` and signed update bundles without credentials.
Protect `main` and restrict creation, updates and deletion of `v*` tags to
release maintainers/automation. Do not force-push published tags.

Keep the public `proto` submodule initialized. Builds use its committed revision
and export the exact protovalidate dependency from `proto/buf.lock`.

## Automatic release flow

1. Merge app changes into `main` using Conventional Commits, such as
   `fix: correct invoice totals` or `feat: add draft tools`. Historical messages
   such as `fix | ...` do not trigger Release Please. Squash-merge PR titles can
   provide the conventional message. The configured package path is `eattura`:
   changes confined to the server or repository tooling do not start app releases.
2. After `.github/workflows/ci.yml` passes (Rust workspace Clippy/tests and
   Angular typecheck/tests/build), Google Release Please opens or updates the
   app release PR, including the changelog. The empty initial manifest and
   `initial-version: 0.1.0` establish the first release; later releases use
   normal semantic version increments.
3. Review and merge the release PR into `main`. Release Please creates the stable
   tag and a **draft** GitHub Release, then calls the desktop build workflow.
   This explicit call is intentional: tags created by `GITHUB_TOKEN` do not
   trigger another push workflow.
4. All four builds must succeed before the final job uploads six installers,
   signed updater packages, `latest.json` and `SHA256SUMS` and publishes the
   release. A failed build leaves it as a draft.
   Rerun failed workflow jobs after fixing infrastructure issues. Code fixes
   require a new version/tag; never move a tag to a different commit.

Release Please updates only `eattura/package.json`,
`eattura/src-tauri/tauri.conf.json`, the explicit app version in
`eattura/src-tauri/Cargo.toml`, the `eattura` package entry in `Cargo.lock`,
`eattura/CHANGELOG.md` and the app's release manifest entry.
The root Cargo workspace and `common`/`server-eattura` versions are independent
and are not bumped. No Rust workspace release plugin is enabled.

## Tag safety and manual tags

Only `vMAJOR.MINOR.PATCH` is accepted, with no leading zeroes other than `0`
itself: for example `v0.1.0` or `v1.2.3`. Uppercase `V`, prereleases, build metadata
and component prefixes are rejected. GitHub's glob is only a prefilter; the
validation script enforces the format.

GitHub cannot combine a tag push filter with a branch filter. Before building,
the workflow verifies that the tagged commit is reachable from `origin/main`
and all four app versions match the tag. Unmerged feature-branch tags are
rejected. Older versions already contained in `main` can still be released.

Maintainers can push annotated stable tags manually after updating and merging
all app version files into `main`. This uses the same validation and build
pipeline. If no draft exists, the publisher creates one with generated notes.
It refuses to modify an already published release.

## Installers and signing

| Platform | Architecture | Packages | Updater artifacts |
| --- | --- | --- | --- |
| macOS | Apple Silicon (ARM64) | DMG | `.app.tar.gz` + `.sig` |
| macOS | Intel (x64) | DMG | `.app.tar.gz` + `.sig` |
| Linux | x64, built on Ubuntu 24.04 | DEB, AppImage | `.AppImage` + `.sig`, `.deb` + `.sig` |
| Windows | x64 | NSIS EXE, MSI | `.exe` + `.sig`, `.msi` + `.sig` |

Artifact names include the target triple to avoid collisions. Linux packages
target the Ubuntu 24.04 runtime baseline; older distributions are not guaranteed
to work. AppImage still requires compatible system dependencies.

Unattended bundling sets `CI=true`. On macOS this avoids Finder AppleScript
customization of the DMG, which can block without GUI automation permissions;
the Tauri `--ci` flag alone does not enable that bundler behavior.

macOS bundles are **ad-hoc signed**, not Developer ID signed or notarized.
Windows installers are **unsigned**. Gatekeeper/SmartScreen warnings are expected.
A verified OS publisher requires a separate Apple/Microsoft certificate setup.
In-app updates are cryptographically verified by `tauri-plugin-updater` against
the Ed25519 `pubkey` in `eattura/src-tauri/tauri.conf.json` with
`requireSignedVersion: true`.

## Local checks

Run from the repository root:

```sh
node --test scripts/release.test.mjs
node scripts/release.mjs check v0.1.0
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cd eattura
bun install --frozen-lockfile
bunx tsc -p tsconfig.spec.json --noEmit
bun run test
bun run build
```

Use the current app version in the version check after the first release.
`task proto` exports locked definitions if `proto/vendor` is absent. Remove that
generated directory before re-exporting after a protobuf lockfile update.

Stable IMAP 2.4 currently pulls `imap-proto 0.10.2`, which reports an upstream
Rust future-incompatibility advisory. Project code passes Clippy with
`-D warnings`; this dependency advisory is not suppressed. Replacing the PEC
transport with a pre-release IMAP API solely to hide it is intentionally avoided.
