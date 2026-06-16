/// Run database migrations.
///
/// Creates the following tables if they don't exist:
///
/// - `clients`: Company/person records (cedente/cessionario)
///   - PK: id (VARCHAR)
///   - Unique: (id_paese, id_codice) for P.IVA deduplication
///
/// - `invoices`: Invoice header records
///   - PK: id (VARCHAR)
///   - FK: cedente_id -> clients(id), cessionario_id -> clients(id)
///   - Unique: (cedente_id, numero, data) per SDI rules
///   - Index: stato for status filtering
///
/// - `invoice_lines`: Line items (DettaglioLinee)
///   - PK: id (VARCHAR)
///   - FK: invoice_id -> invoices(id) ON DELETE CASCADE
///
/// - `invoice_payments`: Payment details (DettaglioPagamento)
///   - PK: id (VARCHAR)
///   - FK: invoice_id -> invoices(id) ON DELETE CASCADE
///
/// - `settings`: Key-value configuration store
pub async fn run_migrations(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS clients (
            id VARCHAR PRIMARY KEY,
            denominazione VARCHAR,
            nome VARCHAR,
            cognome VARCHAR,
            id_paese VARCHAR NOT NULL,
            id_codice VARCHAR NOT NULL,
            codice_fiscale VARCHAR,
            regime_fiscale VARCHAR,
            indirizzo VARCHAR NOT NULL,
            numero_civico VARCHAR,
            cap VARCHAR NOT NULL,
            comune VARCHAR NOT NULL,
            provincia VARCHAR,
            nazione VARCHAR NOT NULL,
            telefono VARCHAR,
            email VARCHAR,
            pec VARCHAR,
            codice_destinatario VARCHAR,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(id_paese, id_codice)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS invoices (
            id VARCHAR PRIMARY KEY,
            numero VARCHAR NOT NULL,
            data VARCHAR NOT NULL,
            tipo_documento VARCHAR NOT NULL,
            divisa VARCHAR NOT NULL,
            importo_totale DOUBLE PRECISION,
            stato VARCHAR NOT NULL DEFAULT 'draft',
            cedente_id VARCHAR NOT NULL REFERENCES clients(id),
            cessionario_id VARCHAR NOT NULL REFERENCES clients(id),
            xml_content TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(cedente_id, numero, data)
        )",
    )
    .execute(pool)
    .await?;

    // Index on stato for efficient status filtering queries.
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_invoices_stato ON invoices(stato)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS invoice_lines (
            id VARCHAR PRIMARY KEY,
            invoice_id VARCHAR NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
            numero_linea INTEGER NOT NULL,
            descrizione VARCHAR NOT NULL,
            quantita DOUBLE PRECISION,
            unita_misura VARCHAR,
            prezzo_unitario DOUBLE PRECISION NOT NULL,
            prezzo_totale DOUBLE PRECISION NOT NULL,
            aliquota_iva DOUBLE PRECISION NOT NULL,
            natura VARCHAR
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS invoice_payments (
            id VARCHAR PRIMARY KEY,
            invoice_id VARCHAR NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
            condizioni_pagamento VARCHAR,
            modalita_pagamento VARCHAR NOT NULL,
            importo_pagamento DOUBLE PRECISION NOT NULL,
            data_scadenza_pagamento VARCHAR,
            iban VARCHAR,
            istituto_finanziario VARCHAR
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS settings (
            key VARCHAR PRIMARY KEY,
            value TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    // SDI notifications received via PEC (RC/NS/MC/NE/DT...).
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sdi_notifications (
            id VARCHAR PRIMARY KEY,
            invoice_id VARCHAR REFERENCES invoices(id) ON DELETE SET NULL,
            tipo VARCHAR NOT NULL,
            identificativo_sdi VARCHAR,
            nome_file VARCHAR,
            esito VARCHAR,
            errori TEXT,
            raw_xml TEXT NOT NULL,
            received_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}
