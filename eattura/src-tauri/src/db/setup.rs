use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;

/// Initialize the SQLite database for the desktop application.
///
/// Creates the database file at the given path if it does not exist,
/// then runs migrations to create all required tables.
///
/// Tables created:
/// - `clients`: Company/person records (cedente/cessionario)
/// - `invoices`: Invoice headers with references to clients
/// - `invoice_lines`: Line items belonging to an invoice
/// - `invoice_payments`: Payment details belonging to an invoice
/// - `settings`: Key-value store for app configuration
pub async fn initialize(db_path: &std::path::Path) -> Result<sqlx::SqlitePool, sqlx::Error> {
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let options = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // Run table creation migrations.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS clients (
            id TEXT PRIMARY KEY,
            denominazione TEXT,
            nome TEXT,
            cognome TEXT,
            id_paese TEXT NOT NULL,
            id_codice TEXT NOT NULL,
            codice_fiscale TEXT,
            regime_fiscale TEXT,
            indirizzo TEXT NOT NULL,
            numero_civico TEXT,
            cap TEXT NOT NULL,
            comune TEXT NOT NULL,
            provincia TEXT,
            nazione TEXT NOT NULL,
            telefono TEXT,
            email TEXT,
            pec TEXT,
            codice_destinatario TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(id_paese, id_codice)
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS invoices (
            id TEXT PRIMARY KEY,
            numero TEXT NOT NULL,
            data TEXT NOT NULL,
            tipo_documento TEXT NOT NULL,
            divisa TEXT NOT NULL,
            importo_totale REAL,
            stato TEXT NOT NULL DEFAULT 'draft',
            cedente_id TEXT NOT NULL REFERENCES clients(id),
            cessionario_id TEXT NOT NULL REFERENCES clients(id),
            xml_content TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(cedente_id, numero, data)
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS invoice_lines (
            id TEXT PRIMARY KEY,
            invoice_id TEXT NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
            numero_linea INTEGER NOT NULL,
            descrizione TEXT NOT NULL,
            quantita REAL,
            unita_misura TEXT,
            prezzo_unitario REAL NOT NULL,
            prezzo_totale REAL NOT NULL,
            aliquota_iva REAL NOT NULL,
            natura TEXT
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS invoice_payments (
            id TEXT PRIMARY KEY,
            invoice_id TEXT NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
            condizioni_pagamento TEXT,
            modalita_pagamento TEXT NOT NULL,
            importo_pagamento REAL NOT NULL,
            data_scadenza_pagamento TEXT,
            iban TEXT,
            istituto_finanziario TEXT
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await?;

    // SDI notifications received via PEC (RC/NS/MC/NE/DT...).
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sdi_notifications (
            id TEXT PRIMARY KEY,
            invoice_id TEXT REFERENCES invoices(id) ON DELETE SET NULL,
            tipo TEXT NOT NULL,
            identificativo_sdi TEXT,
            nome_file TEXT,
            esito TEXT,
            errori TEXT,
            raw_xml TEXT NOT NULL,
            received_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
