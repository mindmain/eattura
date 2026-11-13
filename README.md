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

### Local MCP assistant

Eattura exposes MCP over **stdio**, using its installed executable with `--mcp-stdio`. **Open Eattura and keep it running before connecting your MCP client.** This mode starts no second GUI, archive, or PEC synchronization job. The desktop retains its shared in-memory store. Stdio processes connect to the running app through private OS IPC: Unix domain sockets on macOS/Linux, local named pipes on Windows. There is **no HTTP/TCP MCP listener**, public endpoint, tunnel, or external bridge dependency.

In **Impostazioni app → Assistenti AI · MCP locale**, check stdio status and optionally show client configuration, also written to `~/.eattura/mcp.json`. It contains only the executable path and arguments, not credentials. Internal IPC uses `~/.eattura/mcp-token` and runtime discovery in `~/.eattura/mcp-ipc.json`. Never share these internal files. Mutual nonce-based HMAC-SHA256 authentication prevents connections or impersonation without the secret. Files use owner-only permissions on Unix; sockets sit in random private directories. Windows pipes reject remote clients. There are up to 16 IPC sessions, ten-second authentication/initialization timeouts, 35-second forwarded requests, and 2 MiB MCP message/result limits. A process lock prevents duplicate MCP listeners for the same data directory. MCP failure does not prevent opening the app: inspect activity logs, resolve permissions or duplicate instances, and restart.

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

Authorization covers the current desktop user's invoice archive and draft creation; there are no per-client archive scopes. The stdio process never opens an archive independently. If the app is unavailable it fails with an instruction to open Eattura, without exposing secrets. Stdout is reserved for MCP messages.

Available tools:

| Tool | Allowed operation |
|---|---|
| `get_app_info` | Read app version, configured issuer and usage restrictions, without PEC connection settings. |
| `list_invoices` | Search local invoices by text, status and year with bounded pagination. |
| `get_invoice` | Read invoice details, payments and notification summaries; omit attachment bytes and raw receipt XML. |
| `list_clients` / `get_client` | Find and read existing local client records. |
| `create_draft` | Create a new draft using existing issuer/customer IDs and the desktop invoice request schema. |
| `validate_invoice` | Report local validation errors without changing invoice status. |

**MCP cannot send, issue, promote, edit or delete invoices, change their status, change settings, or access PEC credentials.** New drafts appear in the app through a desktop event. The user must review the draft and personally validate and send it in Eattura. Local validation is not an SDI submission or acceptance. The server's initialization description, title and instructions explicitly state the open-app requirement and the no-transmission policy. When the app is closed, the stdio process cannot connect; the visible registration name and this setup guidance remind the user to open it.

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

## Requisiti

- [Rust](https://www.rust-lang.org/) (edition 2024)
- [Bun](https://bun.sh/)
- [Buf](https://buf.build/) (toolchain protobuf)
- [Task](https://taskfile.dev/)

## Avvio rapido

Tutti i comandi usano [Task](https://taskfile.dev/):

```sh
task install   # Installa le dipendenze Bun + Cargo
task proto     # Esporta le dipendenze protobuf tramite buf
task dev       # Avvia l'app desktop con hot reload
task server    # Avvia il server backend
task build     # Crea la distribuzione di produzione
task icons     # Genera le icone dell'app da un PNG
task clean     # Rimuove gli artefatti di build
task pull      # Aggiorna il git submodule dei proto
```

## Licenza

Distribuito con licenza **Apache 2.0**. Vedi [LICENSE](LICENSE).
