use common::sdi::v1::*;
use common::xml::enum_map::*;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::invoice::{
    CreateInvoiceRequest, InvoiceDetail, InvoiceLineDetail, InvoicePaymentDetail,
};
use crate::services::client_service::ClientService;

/// Business logic for invoice operations.
///
/// Orchestrates between database models, proto types, and validation.
pub struct InvoiceService {
    pub db: sqlx::PgPool,
}

impl InvoiceService {
    /// Create a new invoice from a request, persisting to database.
    ///
    /// Steps: validate request -> insert invoice row -> insert line rows
    /// -> insert payment rows -> return full detail.
    pub async fn create(&self, req: CreateInvoiceRequest) -> Result<InvoiceDetail, AppError> {
        // Verify cedente and cessionario exist.
        let cedente_exists = sqlx::query("SELECT id, COALESCE(denominazione, cognome || ' ' || nome, '') AS display_name FROM clients WHERE id = $1")
            .bind(req.cedente_id.to_string())
            .fetch_optional(&self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Cedente client {} not found", req.cedente_id)))?;

        let cessionario_exists = sqlx::query("SELECT id, COALESCE(denominazione, cognome || ' ' || nome, '') AS display_name FROM clients WHERE id = $1")
            .bind(req.cessionario_id.to_string())
            .fetch_optional(&self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Cessionario client {} not found", req.cessionario_id)))?;

        use sqlx::Row;
        let cedente_denominazione: String = cedente_exists.get("display_name");
        let cessionario_denominazione: String = cessionario_exists.get("display_name");

        let id = Uuid::new_v4();

        // Persist header + lines + payments atomically.
        let mut tx = self.db.begin().await?;

        sqlx::query(
            "INSERT INTO invoices
                (id, numero, data, tipo_documento, divisa, importo_totale, cedente_id, cessionario_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id.to_string())
        .bind(&req.numero)
        .bind(&req.data)
        .bind(&req.tipo_documento)
        .bind(&req.divisa)
        .bind(req.importo_totale)
        .bind(req.cedente_id.to_string())
        .bind(req.cessionario_id.to_string())
        .execute(&mut *tx)
        .await?;

        // Insert line items.
        let mut linee = Vec::new();
        for line in &req.linee {
            let line_id = Uuid::new_v4().to_string();
            let prezzo_totale = line.prezzo_unitario * line.quantita.unwrap_or(1.0);

            sqlx::query(
                "INSERT INTO invoice_lines
                    (id, invoice_id, numero_linea, descrizione, quantita, unita_misura,
                     prezzo_unitario, prezzo_totale, aliquota_iva, natura)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(&line_id)
            .bind(id.to_string())
            .bind(line.numero_linea)
            .bind(&line.descrizione)
            .bind(line.quantita)
            .bind(&line.unita_misura)
            .bind(line.prezzo_unitario)
            .bind(prezzo_totale)
            .bind(line.aliquota_iva)
            .bind(&line.natura)
            .execute(&mut *tx)
            .await?;

            linee.push(InvoiceLineDetail {
                numero_linea: line.numero_linea,
                descrizione: line.descrizione.clone(),
                quantita: line.quantita,
                unita_misura: line.unita_misura.clone(),
                prezzo_unitario: line.prezzo_unitario,
                prezzo_totale,
                aliquota_iva: line.aliquota_iva,
                natura: line.natura.clone(),
            });
        }

        // Insert payment details.
        let mut pagamenti = Vec::new();
        for payment in &req.pagamenti {
            let payment_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO invoice_payments
                    (id, invoice_id, condizioni_pagamento, modalita_pagamento,
                     importo_pagamento, data_scadenza_pagamento, iban, istituto_finanziario)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(&payment_id)
            .bind(id.to_string())
            .bind(&payment.condizioni_pagamento)
            .bind(&payment.modalita_pagamento)
            .bind(payment.importo_pagamento)
            .bind(&payment.data_scadenza_pagamento)
            .bind(&payment.iban)
            .bind(&payment.istituto_finanziario)
            .execute(&mut *tx)
            .await?;

            pagamenti.push(InvoicePaymentDetail {
                modalita_pagamento: payment.modalita_pagamento.clone(),
                importo_pagamento: payment.importo_pagamento,
                data_scadenza_pagamento: payment.data_scadenza_pagamento.clone(),
                iban: payment.iban.clone(),
                istituto_finanziario: payment.istituto_finanziario.clone(),
            });
        }

        tx.commit().await?;

        Ok(InvoiceDetail {
            id,
            numero: req.numero,
            data: req.data,
            tipo_documento: req.tipo_documento,
            divisa: req.divisa,
            importo_totale: req.importo_totale,
            cedente_denominazione,
            cessionario_denominazione,
            stato: "draft".to_string(),
            causale: req.causale,
            linee,
            pagamenti,
        })
    }

    /// Convert a stored invoice into a proto FatturaElettronica for XML export.
    ///
    /// Loads the invoice with all relations (lines, payments, cedente, cessionario)
    /// and maps them to the proto message structure.
    pub async fn to_fattura(&self, id: Uuid) -> Result<FatturaElettronica, AppError> {
        use sqlx::Row;

        // Load invoice header.
        let inv_row = sqlx::query(
            "SELECT i.*, c1.id AS ced_id, c1.denominazione AS ced_denom, c1.nome AS ced_nome,
                    c1.cognome AS ced_cognome, c1.id_paese AS ced_paese, c1.id_codice AS ced_codice,
                    c1.codice_fiscale AS ced_cf, c1.regime_fiscale AS ced_regime,
                    c1.indirizzo AS ced_indirizzo, c1.numero_civico AS ced_civico,
                    c1.cap AS ced_cap, c1.comune AS ced_comune, c1.provincia AS ced_provincia,
                    c1.nazione AS ced_nazione, c1.pec AS ced_pec,
                    c2.denominazione AS cess_denom, c2.nome AS cess_nome,
                    c2.cognome AS cess_cognome, c2.id_paese AS cess_paese,
                    c2.id_codice AS cess_codice, c2.codice_fiscale AS cess_cf,
                    c2.indirizzo AS cess_indirizzo, c2.numero_civico AS cess_civico,
                    c2.cap AS cess_cap, c2.comune AS cess_comune, c2.provincia AS cess_provincia,
                    c2.nazione AS cess_nazione, c2.codice_destinatario AS cess_dest
             FROM invoices i
             JOIN clients c1 ON i.cedente_id = c1.id
             JOIN clients c2 ON i.cessionario_id = c2.id
             WHERE i.id = $1",
        )
        .bind(id.to_string())
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Invoice {id} not found")))?;

        // Load line items.
        let line_rows = sqlx::query(
            "SELECT * FROM invoice_lines WHERE invoice_id = $1 ORDER BY numero_linea",
        )
        .bind(id.to_string())
        .fetch_all(&self.db)
        .await?;

        // Load payment details.
        let payment_rows = sqlx::query(
            "SELECT * FROM invoice_payments WHERE invoice_id = $1",
        )
        .bind(id.to_string())
        .fetch_all(&self.db)
        .await?;

        // Build proto types.
        let tipo_doc_str: String = inv_row.get("tipo_documento");
        let tipo_documento = tipo_documento_from_sdi(&tipo_doc_str).unwrap_or(0);

        let ced_paese: String = inv_row.get("ced_paese");
        let ced_codice: String = inv_row.get("ced_codice");
        let ced_denom: Option<String> = inv_row.get("ced_denom");
        let ced_nome: Option<String> = inv_row.get("ced_nome");
        let ced_cognome: Option<String> = inv_row.get("ced_cognome");
        let ced_cf: Option<String> = inv_row.get("ced_cf");
        let ced_regime: Option<String> = inv_row.get("ced_regime");

        let cess_paese: String = inv_row.get("cess_paese");
        let cess_codice: String = inv_row.get("cess_codice");
        let cess_denom: Option<String> = inv_row.get("cess_denom");
        let cess_nome: Option<String> = inv_row.get("cess_nome");
        let cess_cognome: Option<String> = inv_row.get("cess_cognome");
        let cess_cf: Option<String> = inv_row.get("cess_cf");
        let cess_dest: Option<String> = inv_row.get("cess_dest");

        // Build cedente anagrafica soggetto.
        let ced_soggetto = if let Some(denom) = &ced_denom {
            Some(anagrafica::Soggetto::Denominazione(denom.clone()))
        } else if let (Some(nome), Some(cognome)) = (&ced_nome, &ced_cognome) {
            Some(anagrafica::Soggetto::PersonaFisica(PersonaFisica {
                nome: nome.clone(),
                cognome: cognome.clone(),
            }))
        } else {
            None
        };

        // Build cessionario anagrafica soggetto.
        let cess_soggetto = if let Some(denom) = &cess_denom {
            Some(anagrafica::Soggetto::Denominazione(denom.clone()))
        } else if let (Some(nome), Some(cognome)) = (&cess_nome, &cess_cognome) {
            Some(anagrafica::Soggetto::PersonaFisica(PersonaFisica {
                nome: nome.clone(),
                cognome: cognome.clone(),
            }))
        } else {
            None
        };

        let regime_val = ced_regime
            .as_deref()
            .and_then(regime_fiscale_from_sdi)
            .unwrap_or(10); // RF01 default

        let codice_destinatario = cess_dest.unwrap_or_else(|| "0000000".to_string());

        let header = FatturaElettronicaHeader {
            dati_trasmissione: Some(DatiTrasmissione {
                id_trasmittente: Some(IdFiscale {
                    id_paese: ced_paese.clone(),
                    id_codice: ced_codice.clone(),
                }),
                progressivo_invio: format!("{:05}", 1),
                formato_trasmissione: 10, // FPA12
                codice_destinatario,
                ..Default::default()
            }),
            cedente_prestatore: Some(CedentePrestatore {
                dati_anagrafici: Some(DatiAnagraficiCedente {
                    id_fiscale_iva: Some(IdFiscale {
                        id_paese: ced_paese.clone(),
                        id_codice: ced_codice.clone(),
                    }),
                    codice_fiscale: ced_cf,
                    anagrafica: Some(Anagrafica {
                        soggetto: ced_soggetto,
                        ..Default::default()
                    }),
                    regime_fiscale: regime_val,
                    ..Default::default()
                }),
                sede: Some(Indirizzo {
                    indirizzo: inv_row.get("ced_indirizzo"),
                    numero_civico: inv_row.get("ced_civico"),
                    cap: inv_row.get("ced_cap"),
                    comune: inv_row.get("ced_comune"),
                    provincia: inv_row.get("ced_provincia"),
                    nazione: inv_row.get("ced_nazione"),
                }),
                ..Default::default()
            }),
            cessionario_committente: Some(CessionarioCommittente {
                dati_anagrafici: Some(DatiAnagraficiCessionario {
                    id_fiscale_iva: Some(IdFiscale {
                        id_paese: cess_paese.clone(),
                        id_codice: cess_codice.clone(),
                    }),
                    codice_fiscale: cess_cf,
                    anagrafica: Some(Anagrafica {
                        soggetto: cess_soggetto,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                sede: Some(Indirizzo {
                    indirizzo: inv_row.get("cess_indirizzo"),
                    numero_civico: inv_row.get("cess_civico"),
                    cap: inv_row.get("cess_cap"),
                    comune: inv_row.get("cess_comune"),
                    provincia: inv_row.get("cess_provincia"),
                    nazione: inv_row.get("cess_nazione"),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Build line items.
        let dettaglio_linee: Vec<DettaglioLinee> = line_rows
            .iter()
            .map(|r| {
                let natura_str: Option<String> = r.get("natura");
                DettaglioLinee {
                    numero_linea: r.get("numero_linea"),
                    descrizione: r.get("descrizione"),
                    quantita: r.get("quantita"),
                    prezzo_unitario: r.get("prezzo_unitario"),
                    prezzo_totale: r.get("prezzo_totale"),
                    aliquota_iva: r.get("aliquota_iva"),
                    natura: natura_str.as_deref().and_then(natura_from_sdi),
                    ..Default::default()
                }
            })
            .collect();

        // Compute riepilogo from line items grouped by aliquota_iva.
        let mut riepilogo_map: std::collections::HashMap<String, (f64, f64, Option<i32>)> =
            std::collections::HashMap::new();
        for linea in &dettaglio_linee {
            let key = format!("{:.2}", linea.aliquota_iva);
            let entry = riepilogo_map.entry(key).or_insert((0.0, linea.aliquota_iva, linea.natura));
            entry.0 += linea.prezzo_totale;
        }

        let dati_riepilogo: Vec<DatiRiepilogo> = riepilogo_map
            .values()
            .map(|(imponibile, aliquota, natura)| {
                // Round to 2 decimals (SDI 00421): VAT must match the cent.
                let imposta = ((aliquota * imponibile / 100.0) * 100.0).round() / 100.0;
                DatiRiepilogo {
                    aliquota_iva: *aliquota,
                    imponibile_importo: *imponibile,
                    imposta,
                    natura: *natura,
                    ..Default::default()
                }
            })
            .collect();

        // Build payment details.
        let dati_pagamento: Vec<DatiPagamento> = payment_rows
            .iter()
            .map(|r| {
                let cond_str: Option<String> = r.get("condizioni_pagamento");
                let mod_str: String = r.get("modalita_pagamento");
                DatiPagamento {
                    condizioni_pagamento: cond_str
                        .as_deref()
                        .and_then(condizioni_pagamento_from_sdi)
                        .unwrap_or(20), // TP02 default
                    dettaglio_pagamento: vec![DettaglioPagamento {
                        modalita_pagamento: modalita_pagamento_from_sdi(&mod_str).unwrap_or(50), // MP05 default
                        importo_pagamento: r.get("importo_pagamento"),
                        data_scadenza_pagamento: r.get("data_scadenza_pagamento"),
                        iban: r.get("iban"),
                        istituto_finanziario: r.get("istituto_finanziario"),
                        ..Default::default()
                    }],
                }
            })
            .collect();

        let numero: String = inv_row.get("numero");
        let data: String = inv_row.get("data");
        let divisa: String = inv_row.get("divisa");
        let importo_totale: Option<f64> = inv_row.get("importo_totale");

        let body = FatturaElettronicaBody {
            dati_generali: Some(DatiGenerali {
                dati_generali_documento: Some(DatiGeneraliDocumento {
                    tipo_documento,
                    divisa,
                    data,
                    numero,
                    importo_totale_documento: importo_totale,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            dati_beni_servizi: Some(DatiBeniServizi {
                dettaglio_linee,
                dati_riepilogo,
            }),
            dati_pagamento,
            ..Default::default()
        };

        Ok(FatturaElettronica {
            versione: 10, // FPA12
            sistema_emittente: None,
            header: Some(header),
            body: vec![body],
        })
    }

    /// Import a FatturaElettronica (from XML decode) into the database.
    ///
    /// Maps proto fields to DB models, resolves or creates cedente/cessionario
    /// clients, and persists the full invoice structure.
    pub async fn from_fattura(&self, fattura: FatturaElettronica) -> Result<InvoiceDetail, AppError> {
        let header = fattura
            .header
            .as_ref()
            .ok_or_else(|| AppError::Internal("Missing header in FatturaElettronica".into()))?;

        let body = fattura
            .body
            .first()
            .ok_or_else(|| AppError::Internal("Missing body in FatturaElettronica".into()))?;

        let dg = body
            .dati_generali
            .as_ref()
            .and_then(|dg| dg.dati_generali_documento.as_ref())
            .ok_or_else(|| AppError::Internal("Missing DatiGeneraliDocumento".into()))?;

        let client_svc = ClientService { db: self.db.clone() };

        // Resolve cedente.
        let ced_piva = header
            .cedente_prestatore
            .as_ref()
            .and_then(|cp| cp.dati_anagrafici.as_ref())
            .and_then(|da| da.id_fiscale_iva.as_ref())
            .ok_or_else(|| AppError::Internal("Missing cedente P.IVA".into()))?;

        let cedente_id = client_svc
            .find_or_create_by_piva(&ced_piva.id_paese, &ced_piva.id_codice)
            .await?;

        // Update cedente details if available.
        if let Some(cp) = &header.cedente_prestatore {
            update_client_from_cedente(&self.db, cedente_id, cp).await?;
        }

        // Resolve cessionario.
        let cess_piva = header
            .cessionario_committente
            .as_ref()
            .and_then(|cc| cc.dati_anagrafici.as_ref())
            .and_then(|da| da.id_fiscale_iva.as_ref())
            .ok_or_else(|| AppError::Internal("Missing cessionario P.IVA".into()))?;

        let cessionario_id = client_svc
            .find_or_create_by_piva(&cess_piva.id_paese, &cess_piva.id_codice)
            .await?;

        // Update cessionario details if available.
        if let Some(cc) = &header.cessionario_committente {
            update_client_from_cessionario(&self.db, cessionario_id, cc).await?;
        }

        let tipo_documento_str = tipo_documento_to_sdi(dg.tipo_documento)
            .unwrap_or("TD01")
            .to_string();

        let id = Uuid::new_v4();

        // Persist header + lines + payments atomically.
        let mut tx = self.db.begin().await?;

        sqlx::query(
            "INSERT INTO invoices
                (id, numero, data, tipo_documento, divisa, importo_totale,
                 cedente_id, cessionario_id, stato)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'draft')",
        )
        .bind(id.to_string())
        .bind(&dg.numero)
        .bind(&dg.data)
        .bind(&tipo_documento_str)
        .bind(&dg.divisa)
        .bind(dg.importo_totale_documento)
        .bind(cedente_id.to_string())
        .bind(cessionario_id.to_string())
        .execute(&mut *tx)
        .await?;

        // Insert line items.
        let mut linee = Vec::new();
        if let Some(dbs) = &body.dati_beni_servizi {
            for det in &dbs.dettaglio_linee {
                let line_id = Uuid::new_v4().to_string();
                let natura_str = det.natura.and_then(natura_to_sdi).map(String::from);

                sqlx::query(
                    "INSERT INTO invoice_lines
                        (id, invoice_id, numero_linea, descrizione, quantita, unita_misura,
                         prezzo_unitario, prezzo_totale, aliquota_iva, natura)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(&line_id)
                .bind(id.to_string())
                .bind(det.numero_linea)
                .bind(&det.descrizione)
                .bind(det.quantita)
                .bind(&det.unita_misura)
                .bind(det.prezzo_unitario)
                .bind(det.prezzo_totale)
                .bind(det.aliquota_iva)
                .bind(&natura_str)
                .execute(&mut *tx)
                .await?;

                linee.push(InvoiceLineDetail {
                    numero_linea: det.numero_linea,
                    descrizione: det.descrizione.clone(),
                    quantita: det.quantita,
                    unita_misura: det.unita_misura.clone(),
                    prezzo_unitario: det.prezzo_unitario,
                    prezzo_totale: det.prezzo_totale,
                    aliquota_iva: det.aliquota_iva,
                    natura: natura_str,
                });
            }
        }

        // Insert payment details.
        let mut pagamenti = Vec::new();
        for dp in &body.dati_pagamento {
            for det in &dp.dettaglio_pagamento {
                let payment_id = Uuid::new_v4().to_string();
                let cond_str = condizioni_pagamento_to_sdi(dp.condizioni_pagamento)
                    .unwrap_or("TP02")
                    .to_string();
                let mod_str = modalita_pagamento_to_sdi(det.modalita_pagamento)
                    .unwrap_or("MP05")
                    .to_string();

                sqlx::query(
                    "INSERT INTO invoice_payments
                        (id, invoice_id, condizioni_pagamento, modalita_pagamento,
                         importo_pagamento, data_scadenza_pagamento, iban, istituto_finanziario)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                )
                .bind(&payment_id)
                .bind(id.to_string())
                .bind(&cond_str)
                .bind(&mod_str)
                .bind(det.importo_pagamento)
                .bind(&det.data_scadenza_pagamento)
                .bind(&det.iban)
                .bind(&det.istituto_finanziario)
                .execute(&mut *tx)
                .await?;

                pagamenti.push(InvoicePaymentDetail {
                    modalita_pagamento: mod_str,
                    importo_pagamento: det.importo_pagamento,
                    data_scadenza_pagamento: det.data_scadenza_pagamento.clone(),
                    iban: det.iban.clone(),
                    istituto_finanziario: det.istituto_finanziario.clone(),
                });
            }
        }

        tx.commit().await?;

        // Get display names for cedente/cessionario.
        let ced_name = get_client_display_name(&self.db, cedente_id).await?;
        let cess_name = get_client_display_name(&self.db, cessionario_id).await?;

        Ok(InvoiceDetail {
            id,
            numero: dg.numero.clone(),
            data: dg.data.clone(),
            tipo_documento: tipo_documento_str,
            divisa: dg.divisa.clone(),
            importo_totale: dg.importo_totale_documento,
            cedente_denominazione: ced_name,
            cessionario_denominazione: cess_name,
            stato: "draft".to_string(),
            causale: dg.causale.clone(),
            linee,
            pagamenti,
        })
    }

    /// Update invoice status (draft -> validated -> sent -> accepted/rejected).
    ///
    /// Validates that the status transition is legal before applying it.
    pub async fn update_status(&self, id: Uuid, status: &str) -> Result<(), AppError> {
        use sqlx::Row;

        let row = sqlx::query("SELECT stato FROM invoices WHERE id = $1")
            .bind(id.to_string())
            .fetch_optional(&self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Invoice {id} not found")))?;

        let current: String = row.get("stato");

        // Validate status transition.
        let valid_transition = matches!(
            (current.as_str(), status),
            ("draft", "validated")
                | ("validated", "sent")
                | ("sent", "accepted")
                | ("sent", "rejected")
        );

        if !valid_transition {
            return Err(AppError::Internal(format!(
                "Invalid status transition from '{current}' to '{status}'"
            )));
        }

        sqlx::query("UPDATE invoices SET stato = $2, updated_at = NOW() WHERE id = $1")
            .bind(id.to_string())
            .bind(status)
            .execute(&self.db)
            .await?;

        Ok(())
    }
}

/// Helper to get a client's display name from the database.
async fn get_client_display_name(pool: &sqlx::PgPool, id: Uuid) -> Result<String, AppError> {
    use sqlx::Row;

    let row = sqlx::query(
        "SELECT COALESCE(denominazione, cognome || ' ' || nome, '') AS display_name FROM clients WHERE id = $1",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Client {id} not found")))?;

    Ok(row.get("display_name"))
}

/// Update a client record with data from a CedentePrestatore proto message.
async fn update_client_from_cedente(
    pool: &sqlx::PgPool,
    id: Uuid,
    cp: &CedentePrestatore,
) -> Result<(), AppError> {
    let da = cp.dati_anagrafici.as_ref();
    let sede = cp.sede.as_ref();

    let (denom, nome, cognome) = extract_anagrafica(da.and_then(|d| d.anagrafica.as_ref()));
    let cf = da.and_then(|d| d.codice_fiscale.clone());
    let regime = da
        .map(|d| d.regime_fiscale)
        .and_then(regime_fiscale_to_sdi)
        .map(String::from);

    sqlx::query(
        "UPDATE clients SET
            denominazione = COALESCE($2, denominazione),
            nome = COALESCE($3, nome),
            cognome = COALESCE($4, cognome),
            codice_fiscale = COALESCE($5, codice_fiscale),
            regime_fiscale = COALESCE($6, regime_fiscale),
            indirizzo = COALESCE($7, indirizzo),
            numero_civico = COALESCE($8, numero_civico),
            cap = COALESCE($9, cap),
            comune = COALESCE($10, comune),
            provincia = COALESCE($11, provincia),
            nazione = COALESCE($12, nazione),
            updated_at = NOW()
         WHERE id = $1",
    )
    .bind(id.to_string())
    .bind(&denom)
    .bind(&nome)
    .bind(&cognome)
    .bind(&cf)
    .bind(&regime)
    .bind(sede.map(|s| &s.indirizzo))
    .bind(sede.and_then(|s| s.numero_civico.as_ref()))
    .bind(sede.map(|s| &s.cap))
    .bind(sede.map(|s| &s.comune))
    .bind(sede.and_then(|s| s.provincia.as_ref()))
    .bind(sede.map(|s| &s.nazione))
    .execute(pool)
    .await?;

    Ok(())
}

/// Update a client record with data from a CessionarioCommittente proto message.
async fn update_client_from_cessionario(
    pool: &sqlx::PgPool,
    id: Uuid,
    cc: &CessionarioCommittente,
) -> Result<(), AppError> {
    let da = cc.dati_anagrafici.as_ref();
    let sede = cc.sede.as_ref();

    let (denom, nome, cognome) = extract_anagrafica(da.and_then(|d| d.anagrafica.as_ref()));
    let cf = da.and_then(|d| d.codice_fiscale.clone());

    sqlx::query(
        "UPDATE clients SET
            denominazione = COALESCE($2, denominazione),
            nome = COALESCE($3, nome),
            cognome = COALESCE($4, cognome),
            codice_fiscale = COALESCE($5, codice_fiscale),
            indirizzo = COALESCE($6, indirizzo),
            numero_civico = COALESCE($7, numero_civico),
            cap = COALESCE($8, cap),
            comune = COALESCE($9, comune),
            provincia = COALESCE($10, provincia),
            nazione = COALESCE($11, nazione),
            updated_at = NOW()
         WHERE id = $1",
    )
    .bind(id.to_string())
    .bind(&denom)
    .bind(&nome)
    .bind(&cognome)
    .bind(&cf)
    .bind(sede.map(|s| &s.indirizzo))
    .bind(sede.and_then(|s| s.numero_civico.as_ref()))
    .bind(sede.map(|s| &s.cap))
    .bind(sede.map(|s| &s.comune))
    .bind(sede.and_then(|s| s.provincia.as_ref()))
    .bind(sede.map(|s| &s.nazione))
    .execute(pool)
    .await?;

    Ok(())
}

/// Extract denominazione/nome/cognome from an Anagrafica proto message.
fn extract_anagrafica(
    anagrafica: Option<&Anagrafica>,
) -> (Option<String>, Option<String>, Option<String>) {
    match anagrafica.and_then(|a| a.soggetto.as_ref()) {
        Some(anagrafica::Soggetto::Denominazione(d)) => (Some(d.clone()), None, None),
        Some(anagrafica::Soggetto::PersonaFisica(pf)) => {
            (None, Some(pf.nome.clone()), Some(pf.cognome.clone()))
        }
        None => (None, None, None),
    }
}
