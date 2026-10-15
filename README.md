# Eattura

**Eattura** è un sistema per la gestione delle **fatture elettroniche italiane** (Fattura Elettronica), conforme al protocollo del **SDI** (Sistema di Interscambio) dell'Agenzia delle Entrate.

L'obiettivo è semplificare la **creazione, validazione, invio e gestione** delle fatture elettroniche in modo **indipendente dai fornitori di servizi di fatturazione**: la trasmissione e la ricezione delle fatture avvengono direttamente tramite **PEC** (Posta Elettronica Certificata), senza intermediari.

## Caratteristiche

- **Gestione completa di clienti e fatture** (CRUD e ciclo di vita)
- **Validazione** delle fatture secondo il formato SDI v1.9
- **Invio e ricezione via PEC**, in autonomia rispetto a provider esterni
- **Doppia modalità d'uso**: server headless oppure applicazione desktop locale
- **Job automatici** (cron) per scaricare e inviare le fatture

## Tecnologie

- **Rust** (edition 2024) con runtime asincrono **Tokio**
- **Axum** + **tower-http** per il server HTTP/gRPC
- **SQLx** per l'accesso asincrono al database (PostgreSQL, solo sul server)
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
| `common`         | Libreria condivisa: genera i tipi Rust dai file proto SDI, validazione e utility comuni.     |
| `server-eattura` | Server headless gRPC/HTTP (Axum) con backend **PostgreSQL**. API per fatture, clienti e PEC. |
| `eattura`        | Applicazione desktop **Tauri 2** + Angular, con persistenza locale su **file JSON** e stato in memoria (local-first).    |

### Supporto database

| Contesto | Database   | Driver        |
|----------|------------|---------------|
| Server   | PostgreSQL | sqlx/postgres |
| Desktop  | File JSON  | Serde + stato in memoria |

### Desktop storage

The desktop loads a normalized `index.json` into memory at startup. It contains clients and invoice metadata by ID, links between clients and invoices, lookups by fiscal identity, status and year, and searchable text including invoice line descriptions. Lists and searches use this catalog without opening invoice documents.

Full invoice documents live in `invoices/<revision>.json` and are loaded on demand. Client and invoice catalog IDs for new records use SHA-256 of canonical content with collapsed whitespace; equivalent XML imports resolve to the same invoice ID. These IDs do not rename XML files or change the SDI filename convention. Every creation, XML import, edit or deletion updates the index. Documents and auxiliary state are saved before the index is atomically published; previous revisions are retained. An existing `eattura.json` snapshot is converted when no index exists and is preserved. PostgreSQL remains exclusive to the server.

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

## Stato del progetto

**Sviluppo iniziale.** La struttura del workspace, la pipeline dei proto e lo scaffolding dell'applicazione sono in essere. La logica di business principale (elaborazione fatture, integrazione PEC, modelli del database) è in corso di implementazione.

## Licenza

Distribuito con licenza **Apache 2.0**. Vedi [LICENSE](LICENSE).
