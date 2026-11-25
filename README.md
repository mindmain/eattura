# Eattura

**Eattura** è un sistema per la gestione delle **fatture elettroniche italiane** (Fattura Elettronica), conforme al protocollo del **SDI** (Sistema di Interscambio) dell'Agenzia delle Entrate.

L'obiettivo è semplificare la **creazione, validazione, invio e gestione** delle fatture elettroniche in modo **indipendente dai fornitori di servizi di fatturazione**: la trasmissione e la ricezione delle fatture avvengono direttamente tramite **PEC** (Posta Elettronica Certificata), senza intermediari.

## Caratteristiche

- **Gestione completa di clienti e fatture** (CRUD e ciclo di vita)
- **Validazione** delle fatture secondo il formato SDI v1.9
- **Invio e ricezione via PEC**, in autonomia rispetto a provider esterni
- **Doppia modalità d'uso**: applicazione desktop locale (local-first) e server HTTP stateless
- **Job automatici** (cron) per scaricare le fatture e le ricevute SDI via PEC

## Tecnologie

Desktop release setup and platform/signing limitations are documented in [RELEASING.md](RELEASING.md).

- **Rust** (edition 2024) con runtime asincrono **Tokio**
- **Axum** + **tower-http** per il server HTTP stateless
- **Prost** + **prost-build** per la generazione del codice dai Protocol Buffer
- **Serde** per serializzazione/deserializzazione
- **Tracing** per il logging strutturato
- **Tauri 2** per l'applicazione desktop
- **Angular 21** + **TypeScript 5.9** per il frontend desktop
- **Buf** v2 per il toolchain e la validazione protobuf
- **Bun** come package manager JavaScript

## Architettura

Il progetto è un workspace Cargo composto da tre crate principali:

| Crate            | Descrizione                                                                                  |
|------------------|----------------------------------------------------------------------------------------------|
| `common`         | Libreria condivisa: genera i tipi Rust dai file proto SDI, codec XML, validazione e utility comuni. |
| `server-eattura` | Server HTTP stateless (Axum) senza database. Proxy PEC, lookup VIES e strumenti SDI.         |
| `eattura`        | Applicazione desktop **Tauri 2** + Angular, con persistenza locale su **file JSON** e catalogo in memoria (local-first). |

### Desktop storage

The desktop loads a normalized `index.json` into memory at startup. It contains clients and invoice metadata by ID, links between clients and invoices, lookups by fiscal identity, status and year, and searchable text including invoice line descriptions. Lists and searches use this catalog without opening invoice documents.

Full invoice documents live in `invoices/<revision>.json` and are loaded on demand. Client and invoice catalog IDs for new records use SHA-256 of canonical content with collapsed whitespace; equivalent XML imports resolve to the same invoice ID. These IDs do not rename XML files or change the SDI filename convention. Every creation, XML import, edit or deletion updates the index. Documents and auxiliary state are saved before the index is atomically published; previous revisions are retained. An existing `eattura.json` snapshot is converted when no index exists and is preserved.

### PEC and SdI receipts

PEC synchronization inspects both top-level attachments and nested `postacert.eml` (`message/rfc822`) envelopes, unpacking `.xml` and signed `.xml.p7m` files while ignoring `daticert.xml` transport receipts. Incoming invoices are imported deduplicated; SdI receipts are linked to the matching transmission attempt by `NomeFile` or `IdentificativoSdI` and archived under `ricevute/`.

- `RC` (`RicevutaConsegna`) moves a `sent` invoice to `delivered` (**Consegnata**).
- `MC` (`NotificaMancataConsegna` for PA and `RicevutaImpossibilitaRecapito` for B2B/B2C) attaches the non-delivery receipt to the dossier without marking delivery completed and surfaces the Cassetto Fiscale courtesy-copy notice.
- `NS` (`NotificaScarto` / `RicevutaScarto`) moves the invoice to `rejected` (**Rifiutata**) with 5-day draft rectification.
- `NE` (`NotificaEsito` / `NotificaEsitoCommittente`) and `DT` (`NotificaDecorrenzaTermini` / `DecorrenzaTermini`) advance PA invoices from `sent` or `delivered` to `accepted` (`EC01` / `DT`) or `rejected` (`EC02`, distinct from pre-delivery `NS` discard).
- `AT` (`AttestazioneTrasmissioneFattura`) and `MT` (`MetadatiInvioFile` / `FileMetadati`) are stored in the invoice dossier as informational receipts.

### Local MCP assistant

Eattura exposes MCP over **stdio**, using its installed executable with `--mcp-stdio`. **Open Eattura and keep it running before connecting your MCP client.** This mode starts no second GUI, archive, or PEC synchronization job. The desktop retains its shared in-memory store. Stdio processes connect to the running app through private OS IPC: Unix domain sockets on macOS/Linux, local named pipes on Windows. There is **no HTTP/TCP MCP listener**, public endpoint, tunnel, or external bridge dependency.

In **Impostazioni app → Assistenti AI · MCP locale**, check stdio status and optionally show client configuration, also written to `~/.eattura/mcp.json`. It contains the executable path and arguments. Internal IPC uses `~/.eattura/mcp-token` and runtime discovery in `~/.eattura/mcp-ipc.json`. Keep these internal files private. Mutual nonce-based HMAC-SHA256 authenticates both peers. Files use owner-only permissions on Unix, and sockets sit in random private directories. Windows pipes reject remote clients. There are up to 16 IPC sessions, ten-second authentication and initialization timeouts, 35-second forwarded requests, and 2 MiB MCP message and result limits. The desktop holds an exclusive lock on its local archive. If MCP fails, inspect the app's activity logs.

#### One-button local client setup

Select **Cerca assistenti installati**. Only detected local applications appear; Gemini Web is excluded. Discovery is read-only, never launches clients, and reads standard user configuration files only (up to 1 MiB). Portable, Store, or custom installations may not be detected. Configuration presence does not prove connectivity; project/managed settings can override it.

| Detected client | Direct stdio registration |
| --- | --- |
| Codex CLI / desktop | `command` and `args` in `~/.codex/config.toml`. |
| Claude Code | A user-scoped stdio entry in `~/.claude.json`. |
| Gemini CLI | `command` and `args` in `~/.gemini/settings.json`. |
| Claude Desktop (macOS / Windows) | `command` and `args` in the user's `claude_desktop_config.json`. Automatic Linux setup is not currently supported. |
| ChatGPT desktop | If detected, explains the remote requirement. **Stdio does not enable local MCP in ChatGPT.** No fake install button, guide, or tunnel is offered. |

Read the access warning, close the client, and select **Installa MCP in …**. The single click writes registration directly, without guides or a second in-app confirmation. Every supported client launches the bundled `--mcp-stdio` mode; no tokens are copied into client configuration, and no packages are installed. Requested invoice/customer data may be processed by the client's AI provider. Restart the client, check MCP status, and keep Eattura open. Client-side permissions or enterprise policies may still require approval.

An exact previously generated HTTP entry is migrated when its URL and token match the current app. Identical stdio registrations stay unchanged. Modified/conflicting entries, invalid/oversized files, and symlinks require manual review and are not overwritten. Unrelated settings, other servers, and TOML comments are preserved. Changed existing files receive private sibling `*.eattura-<id>.bak` backups; temporary/new files and backups use `0600` on Unix. Backups may contain old tokens or other secrets: keep them private. Writes compare the original before replacement and serialize Eattura setup operations; close the client to avoid external writers. Custom `CODEX_HOME`, `CLAUDE_CONFIG_DIR`, or `GEMINI_CLI_HOME` values require manual setup. Only configuration paths within the current user's home are accepted.

The registration key is `eattura-open-app-required`. Discovery/install commands are desktop IPC, **not MCP tools**. To undo setup, close the client and restore its backup. Moving Eattura requires reviewing the executable path. To rotate IPC credentials, close Eattura and its MCP clients, remove `~/.eattura/mcp-token`, reopen Eattura, and restart the clients; stdio registration needs no token update. Never automatically retry interrupted draft creation: check the app for a saved draft first.

Implementation references: [Codex MCP](https://developers.openai.com/codex/mcp), [Claude Code MCP](https://code.claude.com/docs/en/mcp), [Claude Desktop configuration](https://github.com/modelcontextprotocol/docs/blob/main/quickstart/user.mdx), [Gemini CLI MCP](https://geminicli.com/docs/tools/mcp-server/), [ChatGPT remote requirements](https://help.openai.com/en/articles/12584461-connecting-custom-mcp-servers-in-chatgpt). Installation buttons do not redirect to these references.

Example JSON (replace the executable path; Claude Code also uses `"type": "stdio"`; Codex uses equivalent TOML):

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

Authorization covers reading the current desktop user's invoice archive and creating or updating drafts. All clients connect to the same archive. The stdio process connects to the running app. Stdout carries MCP messages.

Available tools:

| Tool | Allowed operation |
|---|---|
| `get_app_info` | Read app version, configured issuer and usage restrictions, without PEC connection settings. |
| `list_invoices` | Search local invoices by text, status and year with bounded pagination. |
| `get_invoice` | Read invoice details, payments and notification summaries; omit attachment bytes and raw receipt XML. |
| `list_clients` / `get_client` | Find and read existing local client records. |
| `create_draft` | Create a new draft using existing issuer/customer IDs and the desktop invoice request schema. |
| `update_draft` | Replace editable draft content. Keep the issuer, customer and issuing subject unchanged. Omit `allegati` to keep existing attachments. |
| `update_draft_lines` | Change descriptions and service period dates on existing draft lines in a single transaction. Supply only the fields to change. Use `null` to clear a period date. |
| `validate_invoice` | Report local validation errors without changing invoice status. |

For either update tool, read the draft with `get_invoice` first. Pass its ID and `revision` as `expectedRevision`, and use the returned ID and revision for later calls. For a targeted line update, pass a `linee` array such as `[{"numeroLinea":1,"descrizione":"October support","dataInizioPeriodo":"2026-10-01","dataFinePeriodo":"2026-10-31"}]`.

Drafts are stored locally and appear in the app after creation or editing through MCP. The desktop user reviews the draft, promotes it to validated and initiates any transmission. Use the SdI receipt to check the outcome. If SMTP or the local write leaves an uncertain result, open the invoice and verify the indicated file in the PEC mailbox. A matching SdI receipt can reconcile the pending attempt automatically.

### Automatic backup and sync on Google Drive

Eattura can keep a single structured backup archive (`Eattura/eattura-backup.zip`) synchronized on Google Drive using the non-sensitive `drive.file` scope. Instead of creating multiple dated files, Eattura updates the same archive in place and uses Google Drive's server metadata (`modifiedTime`, `version` and `appProperties.snapshotSha256`) to detect newer backups across devices.

1. In a Google Cloud project, enable the Drive API, configure the OAuth consent screen, and create an OAuth client of type **Desktop app** (publish the consent screen in production mode if you want refresh tokens without the 7-day testing expiry). Download the client JSON. [Google desktop OAuth](https://developers.google.com/identity/protocols/oauth2/native-app)
2. Open **Impostazioni app → Backup automatico su Google Drive**, select the JSON, and click **Collega e avvia backup**. Sign in using the system browser. Eattura uses PKCE and a temporary loopback callback, stores credentials in the OS credential store, and performs the initial backup or restore immediately.
3. Eattura automatically updates `eattura-backup.zip` after any local save (drafts, invoices, clients, payments, settings) and once at the end of bulk operations such as PEC synchronization or folder imports. Every 5 minutes (or on demand via **Sincronizza ora**), Eattura checks Drive metadata and downloads any newer archive, refreshing open views automatically. Opening another Eattura instance with the same Google account and OAuth client automatically discovers and restores the backup.

The desktop communicates directly with Google. The optional Eattura HTTP server exposes its endpoints without built-in authentication. Any network access policy is handled by the deployment.

### Definizioni Protobuf

La cartella `proto/` è un git submodule ([`github.com/mindmain/eattura-proto`](https://github.com/mindmain/eattura-proto)) con le definizioni Protocol Buffer del formato fattura elettronica italiano (SDI v1.9):

| File            | Scopo                                                |
|-----------------|------------------------------------------------------|
| `invoice.proto` | Messaggio radice `FatturaElettronica`                |
| `header.proto`  | Dati di trasmissione e dei soggetti                  |
| `body.proto`    | Dettaglio documento, righe, termini di pagamento     |
| `common.proto`  | Tipi riutilizzabili (IdFiscale, Indirizzo, ecc.)     |
| `enums.proto`   | 18+ enum (tipi documento, regimi fiscali, ecc.)      |
| `natura.proto`  | Codici natura operazione IVA (N1-N7)                 |
| `errors.proto`  | 50+ codici di errore SDI                             |
| `options.proto` | Opzioni protobuf custom per i metadati SDI           |

## Development prerequisites

Install Git, [Rust](https://www.rust-lang.org/), [Bun](https://bun.sh/), [Buf](https://buf.build/) and [Task](https://taskfile.dev/). Install GNU Make to use the Makefile. Use a native runner for each operating system.

- macOS uses Xcode Command Line Tools. Run `xcode-select --install` if they are missing.
- Ubuntu 24.04 uses the same packages as the release runner. Run `sudo apt-get update` followed by `sudo apt-get install --no-install-recommends -y build-essential pkg-config libwebkit2gtk-4.1-dev libssl-dev libdbus-1-dev libayatana-appindicator3-dev librsvg2-dev patchelf`.
- Windows uses the Microsoft C++ Build Tools with the Desktop development with C++ workload and the WebView2 Runtime. Enable the **VBScript** optional feature for MSI builds. [Tauri Windows installer](https://v2.tauri.app/distribute/windows-installer/)

[Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/)

## Run locally

Run these commands from the repository root on macOS, Linux or Windows.

```sh
task install   # Initialize submodules, install locked dependencies and prepare protobuf
task test      # Run Rust tests, TypeScript checks and Angular tests
task dev       # Open the native desktop app with hot reload
task build     # Create production installers for this platform
```

Use `make install`, `make test`, `make dev` and `make build` for the same operations. For a manual check, open the app, create and save a draft, then reopen the app and find it under **Fatture → Bozza**.

## Build in CI with Make

Run one command per native runner. Install the Rust target on that runner with `rustup target add <target>` if needed. The `ci` task runs installation, tests and the Tauri bundler in sequence.

| Runner | Command |
| --- | --- |
| macOS Apple Silicon | `make ci BUILD_TARGET=aarch64-apple-darwin BUILD_BUNDLES=dmg` |
| macOS Intel | `make ci BUILD_TARGET=x86_64-apple-darwin BUILD_BUNDLES=dmg` |
| Linux x64 | `make ci BUILD_TARGET=x86_64-unknown-linux-gnu BUILD_BUNDLES=deb,appimage` |
| Windows x64 | `make ci BUILD_TARGET=x86_64-pc-windows-msvc BUILD_BUNDLES=nsis,msi` |

The installers appear under `target/<target>/release/bundle/` when a target is set. With the default Cargo output path and no explicit target, use `target/release/bundle/`. The existing release workflow handles artifact collection and publishing.

## Licenza

Distribuito con licenza **Apache 2.0**. Vedi [LICENSE](LICENSE).
