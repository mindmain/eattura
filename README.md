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

The desktop app starts an authenticated MCP Streamable HTTP server at `http://127.0.0.1:18765/mcp`. **Open Eattura and keep it running before connecting your MCP client.** The server lives inside the desktop process: closing Eattura stops it; it is not a separate daemon. If a connection fails, open Eattura and retry. A port conflict or configuration error disables MCP without preventing the desktop app from opening; inspect the activity log and restart after resolving the problem.

In **Impostazioni app → Assistenti AI · MCP locale**, check the server status and explicitly reveal the client configuration. The same configuration is written to `~/.eattura/mcp.json`; its bearer token is stored in `~/.eattura/mcp-token`. Both files are created with owner-only permissions on Unix systems. Treat both as private credentials: do not commit, share or paste their contents into a conversation. To rotate the token, close Eattura, remove these two MCP files, reopen the app, and update your authorized clients with the new configuration.

The generated configuration uses the following shape; adapt it to the configuration format of your native MCP client:

```json
{
  "mcpServers": {
    "eattura-open-app-required": {
      "type": "http",
      "url": "http://127.0.0.1:18765/mcp",
      "headers": {
        "Authorization": "Bearer <copy the private token from Eattura settings>"
      }
    }
  }
}
```

Client registration is manual: Eattura does not modify other applications' configuration files. Clients must support Streamable HTTP and a custom Authorization header. The endpoint binds only to IPv4 loopback, rejects browser Origin headers and unrecognized Host headers, limits request bodies to 2 MiB and requests to 30 seconds, and does not enable CORS. It is intended for trusted native clients on the same computer, not browser clients, remote/cloud connectors or other devices. Possession of the token authorizes access to the current desktop user's entire local invoice archive and creation of drafts; there are no separate per-client archive permissions.

Available tools:

| Tool | Allowed operation |
|---|---|
| `get_app_info` | Read app version, configured issuer and usage restrictions, without PEC connection settings. |
| `list_invoices` | Search local invoices by text, status and year with bounded pagination. |
| `get_invoice` | Read invoice details, payments and notification summaries; omit attachment bytes and raw receipt XML. |
| `list_clients` / `get_client` | Find and read existing local client records. |
| `create_draft` | Create a new draft using existing issuer/customer IDs and the desktop invoice request schema. |
| `validate_invoice` | Report local validation errors without changing invoice status. |

**MCP cannot send, issue, promote, edit or delete invoices, change their status, change settings, or access PEC credentials.** New drafts appear in the app through a desktop event. The user must review the draft and personally validate and send it in Eattura. Local validation is not an SDI submission or acceptance. The server's initialization description, title and instructions explicitly state the open-app requirement and the no-transmission policy. When the app is closed, the endpoint cannot return instructions; the visible registration name and this setup guidance remind the user to open it.

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
