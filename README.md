# Eattura

**Eattura** is a local-first platform for **Italian electronic invoicing** (*Fattura Elettronica* / **FatturaPA v1.9**), compliant with the Agenzia delle Entrate **Sistema di Interscambio (SdI)** specifications.

It enables freelancers, *Regime Forfettario* (`RF19`) taxpayers, and businesses to **author, validate, transmit, receive, and archive** electronic invoices **directly via PEC** (*Posta Elettronica Certificata*), without relying on third-party invoicing intermediaries or cloud lock-in.

---

## Key Capabilities

- **Guided FatturaPA & Self-Invoice Editor** — 5-step workflow supporting `TD01..TD27` documents, B2B/B2C (`FPR12`) and Public Administration (`FPA12`, *Split Payment*, `CIG`/`CUP`, procurement orders/contracts, `DDT`, and XML attachments), one-click reverse-charge integration drafts (`TD16..TD21`, `TD27`), social security funds (`DatiCassaPrevidenziale`), withholding taxes (`DatiRitenuta`), and automatic *Regime Forfettario* (`RF19`) + virtual stamp duty (`DatiBollo` over `€77.47`) handling.
- **Offline SdI Validation & Tax Verification** — Local validation of FatturaPA structure and 50+ Agenzia delle Entrate rules (`00001..00476`), Italian VAT (*Partita IVA*) and *Codice Fiscale* check-digit validation, EU **VIES** VAT lookup, and canonical SdI file naming (`<IdPaese><IdCodice>_<ProgressivoInvio>.xml` and `.pdf`).
- **Direct PEC Transmission & Receipt Dossier** — Sends invoices and polls the PEC inbox every 5 minutes without intermediaries, automatically detecting the assigned SdI PEC address (`sdi01@pec.fatturapa.it` → `sdiXX@pec.fatturapa.it`), unpacking nested `postacert.eml` envelopes and signed CAdES `.p7m` files (with X.509 signer inspection), reconciling interrupted SMTP attempts, and tracking every SdI notification (`RC`, `MC`, `NS` with 5-day rectification, PA `NE` `EC01`/`EC02`, `DT`, `AT`, `MT`).
- **Accounting, Courtesy PDF & AgID Preservation** — Generates courtesy PDFs and an official Agenzia delle Entrate layout view, tracks due dates and partial/full payments, computes periodic VAT (*Liquidazione IVA*), and exports Accountant ZIP bundles, *Prima Nota* CSVs (Zucchetti, TeamSystem, Datev, Excel), and **AgID SInCRO** (`UNI 11386:2010`) preservation packages with built-in SHA-256 verification.
- **Private Local MCP Assistant Bridge** — Built-in `--mcp-stdio` mode connecting local AI clients (Codex, Claude Code, Claude Desktop, Gemini CLI) to the open desktop archive over authenticated private OS IPC, restricted to reading records and authoring/validating drafts.
- **Dual Runtime** — Native cross-platform **Tauri 2 + Angular 21** local-first desktop app (`eattura`) with signed GitHub Release updates, plus an optional stateless **Axum** HTTP service (`server-eattura`) for PEC, SdI, and VIES operations.

Desktop release automation, installer targets, and signing setup are documented in [RELEASING.md](RELEASING.md).

---

## Installation

### Quick Install (macOS & Linux)

Install the latest official release with automatic architecture detection (`arm64` Apple Silicon / `x86_64` Intel on macOS, `x86_64` on Linux) and `SHA-256` checksum verification:

```sh
curl -fsSL https://raw.githubusercontent.com/mindmain/eattura/main/scripts/install.sh | sh
```

- **macOS**: Downloads the verified `.app.tar.gz` bundle directly into `/Applications` (or `~/Applications` for non-admin accounts) without browser quarantine attributes, so **Eattura opens immediately without macOS Gatekeeper warnings**. All subsequent updates are verified via Ed25519 and installed directly inside the app.
- **Linux (`x86_64`)**: Installs the verified `.AppImage` to `~/.local/bin/eattura`.
- Pin a specific version or target folder with `EATTURA_VERSION=v0.1.1` or `EATTURA_INSTALL_DIR=/custom/path`.

### Manual Download (macOS, Windows, Linux)

Prebuilt installers (`.dmg`, `.exe`, `.msi`, `.deb`, `.AppImage`) and `SHA256SUMS` are published on [**GitHub Releases**](https://github.com/mindmain/eattura/releases/latest).

> **Note for manual `.dmg` downloads on macOS:** Because builds are ad-hoc signed rather than Apple Developer ID notarized, opening a browser-downloaded `.dmg` for the first time triggers Gatekeeper (*"eattura non è stata aperta"*). Use the `curl` installer above to avoid this completely, or unlock a manual `.dmg` copy once via **System Settings → Privacy & Security → Open Anyway** (or `xattr -cr /Applications/eattura.app`).

---

## Architecture

The repository is a Rust Cargo workspace composed of three crates and a pinned Protobuf submodule:

| Path | Description |
| :--- | :--- |
| `common/` | Shared Rust library: Prost types generated from SdI Protobuf schemas, FatturaPA XML codec, SdI validation rules, courtesy PDF renderer, PEC notification parser, `.p7m` extractor, and VIES client. |
| `eattura/` | Local-first desktop application (**Tauri 2** Rust backend in `eattura/src-tauri` + **Angular 21** frontend in `eattura/src`) with zero-DB JSON persistence in `~/.eattura`, OS keyring credential storage, and local MCP bridge. |
| `server-eattura/` | Stateless **Axum** HTTP server (no database) exposing PEC dispatch/polling, SdI validation/XML conversion, and VIES lookup endpoints. |
| `proto/` | Pinned Git submodule ([`mindmain/eattura-proto`](https://github.com/mindmain/eattura-proto)) defining the FatturaPA v1.9 Protobuf contracts. |

### Technology Stack

- **Backend & Core**: Rust (edition 2024), Tokio, Tauri 2, Axum, Prost, Buf v2, Serde, Tracing
- **Desktop Frontend**: Angular 21 (standalone `OnPush` components & SignalStore), TypeScript 5.9, Chart.js, Vitest, Bun

### Desktop Storage (`~/.eattura`)

At startup, the desktop app acquires an exclusive archive lock in `~/.eattura` and loads a normalized `index.json` catalog into memory. The catalog holds client and invoice metadata, party relationships, lookups by fiscal identity, status, and year, and searchable text including line descriptions—allowing instant list filtering and search without opening individual invoice files.

Full invoice documents live in `invoices/<revision>.json` and are loaded on demand. New client and invoice catalog IDs use a canonical content SHA-256 digest so equivalent XML imports deduplicate deterministically while preserving official SdI filenames. Document revisions and auxiliary records are written atomically before `index.json` is published; legacy `eattura.json` snapshots are migrated non-destructively. PEC passwords remain exclusively in the operating system credential store (macOS Keychain, Windows Credential Manager, Linux Secret Service).

### PEC Synchronization & SdI Lifecycle

PEC synchronization inspects both top-level MIME attachments and nested `postacert.eml` (`message/rfc822`) certified envelopes, extracting `.xml` and signed `.xml.p7m` invoices while ignoring `daticert.xml` transport receipts. Incoming supplier invoices are imported deduplicated; SdI receipts are matched to the corresponding transmission attempt via `NomeFile` or `IdentificativoSdI`:

- `RC` (*RicevutaConsegna*) transitions a `sent` invoice to `delivered` (**Consegnata**).
- `MC` (*NotificaMancataConsegna* for PA / *RicevutaImpossibilitaRecapito* for B2B/B2C) attaches the non-delivery notice to the dossier and surfaces the *Cassetto Fiscale* availability notice without marking delivery complete.
- `NS` (*NotificaScarto* / *RicevutaScarto*) transitions the invoice to `rejected` (**Rifiutata**) and enables the 5-day draft rectification workflow.
- `NE` (*NotificaEsito*) and `DT` (*DecorrenzaTermini*) advance PA invoices to `accepted` (`EC01` / `DT`) or post-delivery `rejected` (`EC02`, kept distinct and locked from `NS` discards).
- `AT` (*AttestazioneTrasmissioneFattura*) and `MT` (*MetadatiInvioFile* / *FileMetadati*) are archived in the invoice dossier.

---

## Local MCP Assistant

Eattura exposes a Model Context Protocol (MCP) server over **stdio** using its installed binary with `--mcp-stdio`. **Open Eattura and keep it running before connecting an MCP client.** Stdio processes start no second GUI, archive lock, or PEC job; they connect to the running app over private OS IPC (Unix domain sockets on macOS/Linux, local named pipes rejecting remote clients on Windows) authenticated via mutual nonce-based HMAC-SHA256 (`~/.eattura/mcp-token` and `~/.eattura/mcp-ipc.json`, mode `0600`). There is no HTTP/TCP MCP listener or external bridge dependency.

### One-Button Client Setup

In **Impostazioni app → Assistenti AI · MCP locale**, select **Cerca assistenti installati** and click **Installa MCP in …** after closing the target client:

| Detected Client | Direct Stdio Registration |
| :--- | :--- |
| Codex CLI / desktop | `command` and `args` in `~/.codex/config.toml` |
| Claude Code | User-scoped `stdio` entry in `~/.claude.json` |
| Gemini CLI | `command` and `args` in `~/.gemini/settings.json` |
| Claude Desktop (macOS / Windows) | `command` and `args` in `claude_desktop_config.json` |
| ChatGPT desktop | Explains remote MCP requirement (local stdio is not supported by ChatGPT) |

Existing unrelated configuration and TOML comments are preserved, and modified files receive private `*.eattura-<id>.bak` backups before atomic replacement. Manual configuration uses the server key `eattura-open-app-required`:

```json
{
  "mcpServers": {
    "eattura-open-app-required": {
      "command": "/absolute/path/to/eattura",
      "args": ["--mcp-stdio"]
    }
  }
}
```

### Exposed MCP Tools

MCP clients can read the local archive and create or update drafts, but **cannot transmit PEC messages, delete records, or alter SdI lifecycle states**:

| Tool | Allowed Operation |
| :--- | :--- |
| `get_app_info` | Read app version, configured issuer, and usage boundaries (excludes PEC credentials). |
| `list_invoices` | Search local invoices by text, status, and year with bounded pagination. |
| `get_invoice` | Read full invoice details, lines, payments, and SdI receipt summaries. |
| `list_clients` / `get_client` | Search and read local client/supplier records. |
| `create_draft` | Create a new draft invoice referencing existing catalog client IDs. |
| `update_draft` | Update editable draft fields using optimistic `expectedRevision` concurrency. |
| `update_draft_lines` | Patch line descriptions and service period dates (`dataInizioPeriodo`, `dataFinePeriodo`) atomically. |
| `validate_invoice` | Run local FatturaPA / SdI validation rules without changing invoice status. |

---

## Protobuf Schemas (`proto/`)

The `proto/` submodule contains the typed FatturaPA v1.9 definitions compiled by `common/build.rs`:

| File | Purpose |
| :--- | :--- |
| `invoice.proto` | Root `FatturaElettronica` message |
| `header.proto` | Transmission metadata (`DatiTrasmissione`) and party headers |
| `body.proto` | General document data, detail lines, VAT summaries, and payment terms |
| `common.proto` | Shared value types (`IdFiscale`, `Indirizzo`, `Anagrafica`) |
| `enums.proto` | FatturaPA enumerations (document types, tax regimes, payment methods, funds) |
| `natura.proto` | VAT exemption / non-taxable nature codes (`N1..N7`) |
| `errors.proto` | SdI validation error catalog |
| `options.proto` | Custom Protobuf options for SdI metadata |

---

## Development Prerequisites

Install Git, [Rust](https://www.rust-lang.org/), [Bun](https://bun.sh/), [Buf](https://buf.build/), and [Task](https://taskfile.dev/) (or GNU Make):

- **macOS**: Xcode Command Line Tools (`xcode-select --install`).
- **Ubuntu 24.04**:
  ```sh
  sudo apt-get update && sudo apt-get install --no-install-recommends -y \
    build-essential pkg-config libwebkit2gtk-4.1-dev libssl-dev \
    libdbus-1-dev libayatana-appindicator3-dev librsvg2-dev patchelf
  ```
- **Windows**: Microsoft C++ Build Tools (*Desktop development with C++* workload), WebView2 Runtime, and the optional **VBScript** feature for `.msi` installers.

---

## Run & Build Locally

Run from the repository root on macOS, Linux, or Windows (`make install`, `make test`, `make dev`, and `make build` provide identical targets):

```sh
task install   # Initialize submodules, install locked dependencies, and export protobufs
task test      # Run Rust workspace tests, TypeScript checks, and Angular Vitest suites
task dev       # Launch the native desktop app in development mode with hot reload
task build     # Build production desktop installers for the current OS
```

### Native CI Build Targets

| Runner | Command |
| :--- | :--- |
| macOS Apple Silicon | `make ci BUILD_TARGET=aarch64-apple-darwin BUILD_BUNDLES=app,dmg` |
| macOS Intel | `make ci BUILD_TARGET=x86_64-apple-darwin BUILD_BUNDLES=app,dmg` |
| Linux x64 | `make ci BUILD_TARGET=x86_64-unknown-linux-gnu BUILD_BUNDLES=deb,appimage` |
| Windows x64 | `make ci BUILD_TARGET=x86_64-pc-windows-msvc BUILD_BUNDLES=nsis,msi` |

---

## License

Licensed under the **Apache License, Version 2.0**. See [LICENSE](LICENSE).
