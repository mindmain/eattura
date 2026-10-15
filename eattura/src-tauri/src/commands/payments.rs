use std::collections::HashMap;
use std::path::PathBuf;
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::commands::invoices::{fetch_invoice_detail, list_invoices_store};
use crate::storage::FileStore;
use crate::TauriState;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PaymentsLedger {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub entries: HashMap<String, InvoiceLedgerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceLedgerEntry {
    pub invoice_id: String,
    pub saldo_residuo: f64,
    pub pagata: bool,
    #[serde(default)]
    pub pagamenti: Vec<PaymentRecord>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRecord {
    pub id: String,
    pub data_pagamento: String, // YYYY-MM-DD
    pub importo: f64,
    pub modalita: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentScheduleItem {
    pub invoice_id: String,
    pub numero: String,
    pub data_documento: String,
    pub direction: String, // "outbound" | "inbound"
    pub controparte_nome: String,
    pub controparte_id: String,
    pub importo_totale: f64,
    pub importo_pagato: f64,
    pub saldo_residuo: f64,
    pub pagata: bool,
    pub data_scadenza: String,
    pub data_ultimo_pagamento: Option<String>,
    pub modalita_pagamento: Option<String>,
    pub giorni_ritardo: i64,
    pub stato_scadenza: String, // "scaduta" | "in_scadenza" | "futura" | "saldata"
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleFilter {
    pub direction: Option<String>, // "outbound" | "inbound" | null
    pub stato: Option<String>,     // "scaduta" | "in_scadenza" | "futura" | "saldata" | null
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterPaymentRequest {
    pub invoice_id: String,
    pub importo: f64,
    pub data_pagamento: String, // YYYY-MM-DD
    pub modalita: Option<String>,
    pub note: Option<String>,
    #[serde(default)]
    pub salda_tutto: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IvaLiquidationRequest {
    pub year: i32,
    pub period: String, // "M01".."M12", "Q1".."Q4"
    #[serde(default)]
    pub credito_precedente: f64,
    #[serde(default)]
    pub is_trimestrale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IvaAliquotaDettaglio {
    pub aliquota: f64,
    pub imponibile_vendite: f64,
    pub imposta_vendite: f64,
    pub imponibile_acquisti: f64,
    pub imposta_acquisti: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IvaLiquidationResult {
    pub year: i32,
    pub period: String,
    pub periodo_label: String,
    pub data_scadenza_versamento: String,
    pub codice_tributo_f24: String,
    pub imponibile_vendite: f64,
    pub iva_vendite_totale: f64,
    pub iva_vendite_split_payment: f64,
    pub iva_vendite_esigibile: f64,
    pub imponibile_acquisti: f64,
    pub iva_acquisti_detraibile: f64,
    pub credito_iva_precedente: f64,
    pub saldo_iva_periodo: f64,
    pub interessi_trimestrali_1_perc: f64,
    pub totale_da_versare_f24: f64,
    pub credito_da_riportare: f64,
    pub versamento_sotto_soglia_100_euro: bool,
    pub note_fiscali: Vec<String>,
    pub aliquote_dettaglio: Vec<IvaAliquotaDettaglio>,
}

fn ledger_path(store: &FileStore) -> PathBuf {
    store.data_dir().join("state").join("payments_ledger.json")
}

fn load_ledger(store: &FileStore) -> PaymentsLedger {
    let path = ledger_path(store);
    if path.exists() {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(ledger) = serde_json::from_slice::<PaymentsLedger>(&bytes) {
                return ledger;
            }
        }
    }
    PaymentsLedger {
        version: 1,
        entries: HashMap::new(),
    }
}

fn save_ledger(store: &FileStore, ledger: &PaymentsLedger) -> Result<(), String> {
    let path = ledger_path(store);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(ledger).map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Retrieve the payment schedule across all invoices in the vault.
#[tauri::command]
pub async fn get_payment_schedule(
    state: tauri::State<'_, TauriState>,
    filter: Option<ScheduleFilter>,
) -> Result<Vec<PaymentScheduleItem>, String> {
    get_payment_schedule_store(&state.store, filter).await
}

pub async fn get_payment_schedule_store(
    store: &FileStore,
    filter: Option<ScheduleFilter>,
) -> Result<Vec<PaymentScheduleItem>, String> {
    let ledger = load_ledger(store);
    let summaries = list_invoices_store(store).await?;
    let today = Utc::now().naive_utc().date();

    let filter = filter.unwrap_or_default();
    let mut items = Vec::new();

    for s in summaries {
        // Skip drafts as they are not definitive accounting documents
        if s.stato == "draft" {
            continue;
        }

        let Ok(detail) = fetch_invoice_detail(store, &s.id).await else {
            continue;
        };

        let direction_norm = match s.direction.as_str() {
            "sale" | "outbound" => "outbound",
            "purchase" | "inbound" => "inbound",
            _ => "outbound",
        };

        if let Some(ref dir) = filter.direction {
            if dir != "all" && dir != direction_norm {
                continue;
            }
        }

        let (controparte_nome, controparte_id) = if direction_norm == "outbound" {
            (detail.cessionario_denominazione.clone(), detail.cessionario_id.clone())
        } else {
            (detail.cedente_denominazione.clone(), detail.cedente_id.clone())
        };

        let total_amount = detail
            .netto_a_pagare
            .or(detail.importo_totale)
            .unwrap_or(0.0);

        // Due date resolution
        let due_date_str = detail
            .pagamenti
            .iter()
            .find_map(|p| p.data_scadenza_pagamento.clone())
            .unwrap_or_else(|| {
                // Default to invoice date + 30 days
                if let Ok(inv_date) = NaiveDate::parse_from_str(&detail.data, "%Y-%m-%d") {
                    (inv_date + chrono::Duration::days(30)).format("%Y-%m-%d").to_string()
                } else {
                    detail.data.clone()
                }
            });

        let modalita_pagamento = detail
            .pagamenti
            .first()
            .map(|p| p.modalita_pagamento.clone());

        let ledger_entry = ledger.entries.get(&s.id);
        let importo_pagato = ledger_entry
            .map(|e| e.pagamenti.iter().map(|p| p.importo).sum())
            .unwrap_or(0.0);

        let saldo_residuo = (total_amount - importo_pagato).max(0.0);
        let pagata = ledger_entry
            .map(|e| e.pagata || saldo_residuo <= 0.001)
            .unwrap_or(false);

        let data_ultimo_pagamento = ledger_entry
            .and_then(|e| e.pagamenti.last())
            .map(|p| p.data_pagamento.clone());

        let note = ledger_entry.and_then(|e| e.note.clone());

        // Status and delay computation
        let parsed_due_date = NaiveDate::parse_from_str(&due_date_str, "%Y-%m-%d")
            .unwrap_or(today);

        let (stato_scadenza, giorni_ritardo) = if pagata {
            ("saldata".to_string(), 0)
        } else if today > parsed_due_date {
            let delay = (today - parsed_due_date).num_days();
            ("scaduta".to_string(), delay)
        } else if parsed_due_date <= today + chrono::Duration::days(7) {
            ("in_scadenza".to_string(), 0)
        } else {
            ("futura".to_string(), 0)
        };

        if let Some(ref st) = filter.stato {
            if st != "all" && st != &stato_scadenza {
                continue;
            }
        }

        items.push(PaymentScheduleItem {
            invoice_id: s.id,
            numero: s.numero,
            data_documento: s.data,
            direction: direction_norm.to_string(),
            controparte_nome,
            controparte_id,
            importo_totale: (total_amount * 100.0).round() / 100.0,
            importo_pagato: (importo_pagato * 100.0).round() / 100.0,
            saldo_residuo: (saldo_residuo * 100.0).round() / 100.0,
            pagata,
            data_scadenza: due_date_str,
            data_ultimo_pagamento,
            modalita_pagamento,
            giorni_ritardo,
            stato_scadenza,
            note,
        });
    }

    // Sort: "scaduta" (greatest delay first), then "in_scadenza", then "futura", then "saldata"
    items.sort_by(|a, b| {
        let rank = |st: &str| match st {
            "scaduta" => 1,
            "in_scadenza" => 2,
            "futura" => 3,
            "saldata" => 4,
            _ => 5,
        };
        let rank_a = rank(&a.stato_scadenza);
        let rank_b = rank(&b.stato_scadenza);

        if rank_a != rank_b {
            rank_a.cmp(&rank_b)
        } else if a.stato_scadenza == "scaduta" {
            b.giorni_ritardo.cmp(&a.giorni_ritardo)
        } else {
            a.data_scadenza.cmp(&b.data_scadenza)
        }
    });

    Ok(items)
}

/// Register a full or partial payment against an invoice in the sovereign sidecar ledger.
#[tauri::command]
pub async fn register_invoice_payment(
    state: tauri::State<'_, TauriState>,
    request: RegisterPaymentRequest,
) -> Result<PaymentScheduleItem, String> {
    register_invoice_payment_store(&state.store, request).await
}

pub async fn register_invoice_payment_store(
    store: &FileStore,
    request: RegisterPaymentRequest,
) -> Result<PaymentScheduleItem, String> {
    let detail = fetch_invoice_detail(store, &request.invoice_id).await?;
    let mut ledger = load_ledger(store);

    let total_amount = detail
        .netto_a_pagare
        .or(detail.importo_totale)
        .unwrap_or(0.0);

    let entry = ledger
        .entries
        .entry(request.invoice_id.clone())
        .or_insert_with(|| InvoiceLedgerEntry {
            invoice_id: request.invoice_id.clone(),
            saldo_residuo: total_amount,
            pagata: false,
            pagamenti: Vec::new(),
            note: None,
        });

    let payment_amount = if request.salda_tutto {
        let already_paid: f64 = entry.pagamenti.iter().map(|p| p.importo).sum();
        (total_amount - already_paid).max(0.0)
    } else {
        request.importo
    };

    if payment_amount > 0.0 {
        entry.pagamenti.push(PaymentRecord {
            id: uuid::Uuid::new_v4().to_string(),
            data_pagamento: request.data_pagamento.clone(),
            importo: (payment_amount * 100.0).round() / 100.0,
            modalita: request.modalita,
            note: request.note.clone(),
        });
    }

    if let Some(n) = request.note {
        entry.note = Some(n);
    }

    let total_paid: f64 = entry.pagamenti.iter().map(|p| p.importo).sum();
    let residual = (total_amount - total_paid).max(0.0);
    entry.saldo_residuo = (residual * 100.0).round() / 100.0;
    entry.pagata = request.salda_tutto || entry.saldo_residuo <= 0.001;

    save_ledger(store, &ledger)?;

    // Return the updated item
    let schedule = get_payment_schedule_store(store, None).await?;
    schedule
        .into_iter()
        .find(|item| item.invoice_id == request.invoice_id)
        .ok_or_else(|| "Impossibile trovare elemento nello scadenzario".to_string())
}

/// Retrieve ledger payment records history for an invoice.
#[tauri::command]
pub async fn get_invoice_payment_history(
    state: tauri::State<'_, TauriState>,
    invoice_id: String,
) -> Result<Option<InvoiceLedgerEntry>, String> {
    let ledger = load_ledger(&state.store);
    Ok(ledger.entries.get(&invoice_id).cloned())
}

/// Calculate Periodic VAT Liquidation (Liquidazione IVA) based strictly on Italian fiscal law:
/// - Sales VAT (IVA a debito) minus Split Payment (Esigibilità 'S' non a debito)
/// - Purchases VAT (IVA a credito detraibile)
/// - Deduction of previous period VAT credit
/// - 1% interest surcharge for quarterly taxpayers (Q1-Q3)
/// - Minimum payment threshold (100.00 € minimum pursuant to art. 1 comma 4 D.Lgs 127/2015)
#[tauri::command]
pub async fn calculate_iva_liquidation(
    state: tauri::State<'_, TauriState>,
    request: IvaLiquidationRequest,
) -> Result<IvaLiquidationResult, String> {
    calculate_iva_liquidation_store(&state.store, request).await
}

pub async fn calculate_iva_liquidation_store(
    store: &FileStore,
    request: IvaLiquidationRequest,
) -> Result<IvaLiquidationResult, String> {
    let (start_date, end_date, periodo_label, codice_tributo_f24, data_scadenza) =
        match request.period.to_uppercase().as_str() {
            "M01" => (
                format!("{}-01-01", request.year),
                format!("{}-01-31", request.year),
                format!("Gennaio {}", request.year),
                "6001".to_string(),
                format!("{}-02-16", request.year),
            ),
            "M02" => {
                let leap = (request.year % 4 == 0 && request.year % 100 != 0) || (request.year % 400 == 0);
                let last_feb = if leap { 29 } else { 28 };
                (
                    format!("{}-02-01", request.year),
                    format!("{}-02-{last_feb}", request.year),
                    format!("Febbraio {}", request.year),
                    "6002".to_string(),
                    format!("{}-03-16", request.year),
                )
            }
            "M03" => (
                format!("{}-03-01", request.year),
                format!("{}-03-31", request.year),
                format!("Marzo {}", request.year),
                "6003".to_string(),
                format!("{}-04-16", request.year),
            ),
            "M04" => (
                format!("{}-04-01", request.year),
                format!("{}-04-30", request.year),
                format!("Aprile {}", request.year),
                "6004".to_string(),
                format!("{}-05-16", request.year),
            ),
            "M05" => (
                format!("{}-05-01", request.year),
                format!("{}-05-31", request.year),
                format!("Maggio {}", request.year),
                "6005".to_string(),
                format!("{}-06-16", request.year),
            ),
            "M06" => (
                format!("{}-06-01", request.year),
                format!("{}-06-30", request.year),
                format!("Giugno {}", request.year),
                "6006".to_string(),
                format!("{}-07-16", request.year),
            ),
            "M07" => (
                format!("{}-07-01", request.year),
                format!("{}-07-31", request.year),
                format!("Luglio {}", request.year),
                "6007".to_string(),
                format!("{}-08-20", request.year), // Proroga feriale
            ),
            "M08" => (
                format!("{}-08-01", request.year),
                format!("{}-08-31", request.year),
                format!("Agosto {}", request.year),
                "6008".to_string(),
                format!("{}-09-16", request.year),
            ),
            "M09" => (
                format!("{}-09-01", request.year),
                format!("{}-09-30", request.year),
                format!("Settembre {}", request.year),
                "6009".to_string(),
                format!("{}-10-16", request.year),
            ),
            "M10" => (
                format!("{}-10-01", request.year),
                format!("{}-10-31", request.year),
                format!("Ottobre {}", request.year),
                "6010".to_string(),
                format!("{}-11-16", request.year),
            ),
            "M11" => (
                format!("{}-11-01", request.year),
                format!("{}-11-30", request.year),
                format!("Novembre {}", request.year),
                "6011".to_string(),
                format!("{}-12-16", request.year),
            ),
            "M12" => (
                format!("{}-12-01", request.year),
                format!("{}-12-31", request.year),
                format!("Dicembre {}", request.year),
                "6012".to_string(),
                format!("{}-01-16", request.year + 1),
            ),
            "Q1" => (
                format!("{}-01-01", request.year),
                format!("{}-03-31", request.year),
                format!("1° Trimestre (Gen-Mar) {}", request.year),
                "6031".to_string(),
                format!("{}-05-16", request.year),
            ),
            "Q2" => (
                format!("{}-04-01", request.year),
                format!("{}-06-30", request.year),
                format!("2° Trimestre (Apr-Giu) {}", request.year),
                "6032".to_string(),
                format!("{}-08-20", request.year), // Proroga feriale
            ),
            "Q3" => (
                format!("{}-07-01", request.year),
                format!("{}-09-30", request.year),
                format!("3° Trimestre (Lug-Set) {}", request.year),
                "6033".to_string(),
                format!("{}-11-16", request.year),
            ),
            "Q4" => (
                format!("{}-10-01", request.year),
                format!("{}-12-31", request.year),
                format!("4° Trimestre (Ott-Dic) {}", request.year),
                "6099".to_string(),
                format!("{}-03-16", request.year + 1), // Confluisce in saldo annuale
            ),
            other => return Err(format!("Periodo di liquidazione non valido: '{other}'")),
        };

    let summaries = list_invoices_store(store).await?;
    let mut rate_map: HashMap<i64, (f64, f64, f64, f64)> = HashMap::new(); // key: aliquota * 100 -> (imp_v, tax_v, imp_a, tax_a)

    let mut imponibile_vendite = 0.0;
    let mut iva_vendite_totale = 0.0;
    let mut iva_vendite_split_payment = 0.0;
    let mut iva_vendite_esigibile = 0.0;

    let mut imponibile_acquisti = 0.0;
    let mut iva_acquisti_detraibile = 0.0;

    for s in summaries {
        if s.data < start_date || s.data > end_date {
            continue;
        }
        if s.stato == "draft" || s.stato == "rejected" {
            continue;
        }

        let Ok(detail) = fetch_invoice_detail(store, &s.id).await else {
            continue;
        };

        let direction_norm = match s.direction.as_str() {
            "sale" | "outbound" => "outbound",
            "purchase" | "inbound" => "inbound",
            _ => "outbound",
        };

        let is_split = detail.is_split_payment
            || detail.esigibilita_iva.as_deref() == Some("S")
            || detail.cessionario_denominazione.to_lowercase().contains("comune")
            || detail.cessionario_denominazione.to_lowercase().contains("ministero")
            || detail.cessionario_denominazione.to_lowercase().contains("asl");

        if direction_norm == "outbound" {
            for line in &detail.linee {
                let tax = (line.prezzo_totale * line.aliquota_iva / 100.0 * 100.0).round() / 100.0;
                imponibile_vendite += line.prezzo_totale;
                iva_vendite_totale += tax;

                let rate_key = (line.aliquota_iva * 100.0).round() as i64;
                let entry = rate_map.entry(rate_key).or_insert((0.0, 0.0, 0.0, 0.0));
                entry.0 += line.prezzo_totale;
                entry.1 += tax;

                if is_split {
                    iva_vendite_split_payment += tax;
                } else {
                    iva_vendite_esigibile += tax;
                }
            }
        } else {
            for line in &detail.linee {
                let tax = (line.prezzo_totale * line.aliquota_iva / 100.0 * 100.0).round() / 100.0;
                imponibile_acquisti += line.prezzo_totale;
                iva_acquisti_detraibile += tax;

                let rate_key = (line.aliquota_iva * 100.0).round() as i64;
                let entry = rate_map.entry(rate_key).or_insert((0.0, 0.0, 0.0, 0.0));
                entry.2 += line.prezzo_totale;
                entry.3 += tax;
            }
        }
    }

    let mut aliquote_dettaglio: Vec<IvaAliquotaDettaglio> = rate_map
        .into_iter()
        .map(|(key, (iv, tv, ia, ta))| IvaAliquotaDettaglio {
            aliquota: (key as f64) / 100.0,
            imponibile_vendite: (iv * 100.0).round() / 100.0,
            imposta_vendite: (tv * 100.0).round() / 100.0,
            imponibile_acquisti: (ia * 100.0).round() / 100.0,
            imposta_acquisti: (ta * 100.0).round() / 100.0,
        })
        .collect();
    aliquote_dettaglio.sort_by(|a, b| b.aliquota.partial_cmp(&a.aliquota).unwrap());

    // Basic calculation:
    // Saldo = IVA esigibile vendite - IVA detraibile acquisti - Credito precedente
    let saldo_iva_periodo = ((iva_vendite_esigibile - iva_acquisti_detraibile - request.credito_precedente) * 100.0).round() / 100.0;

    let mut note_fiscali = Vec::new();

    if iva_vendite_split_payment > 0.0 {
        note_fiscali.push(format!(
            "Stornati € {:.2} di IVA da split payment verso enti pubblici (non concorre al versamento ai sensi dell'art. 17-ter D.P.R. 633/72).",
            iva_vendite_split_payment
        ));
    }

    let (interessi_trimestrali_1_perc, totale_da_versare_f24, credito_da_riportare, versamento_sotto_soglia_100_euro) =
        if saldo_iva_periodo > 0.0 {
            let int = if request.is_trimestrale && request.period != "Q4" {
                let interest = (saldo_iva_periodo * 0.01 * 100.0).round() / 100.0;
                note_fiscali.push(format!(
                    "Applicata maggiorazione dell'1% a titolo di interessi per regime trimestrale (+ € {:.2}).",
                    interest
                ));
                interest
            } else {
                0.0
            };

            let debito_finale = ((saldo_iva_periodo + int) * 100.0).round() / 100.0;

            if debito_finale <= 100.0 {
                note_fiscali.push(
                    "Importo a debito pari o inferiore a 100,00 €. Ai sensi dell'art. 1 c. 4 D.Lgs 127/2015 il versamento è sospeso e verrà sommato al debito del periodo successivo.".to_string(),
                );
                (int, 0.0, 0.0, true)
            } else {
                (int, debito_finale, 0.0, false)
            }
        } else {
            let credito = (-saldo_iva_periodo * 100.0).round() / 100.0;
            if credito > 0.0 {
                note_fiscali.push(format!(
                    "Posizione a credito IVA per € {:.2}. L'importo può essere riportato in detrazione nella liquidazione successiva o compensato.",
                    credito
                ));
            }
            (0.0, 0.0, credito, false)
        };

    Ok(IvaLiquidationResult {
        year: request.year,
        period: request.period,
        periodo_label,
        data_scadenza_versamento: data_scadenza,
        codice_tributo_f24,
        imponibile_vendite: (imponibile_vendite * 100.0).round() / 100.0,
        iva_vendite_totale: (iva_vendite_totale * 100.0).round() / 100.0,
        iva_vendite_split_payment: (iva_vendite_split_payment * 100.0).round() / 100.0,
        iva_vendite_esigibile: (iva_vendite_esigibile * 100.0).round() / 100.0,
        imponibile_acquisti: (imponibile_acquisti * 100.0).round() / 100.0,
        iva_acquisti_detraibile: (iva_acquisti_detraibile * 100.0).round() / 100.0,
        credito_iva_precedente: (request.credito_precedente * 100.0).round() / 100.0,
        saldo_iva_periodo,
        interessi_trimestrali_1_perc,
        totale_da_versare_f24,
        credito_da_riportare,
        versamento_sotto_soglia_100_euro,
        note_fiscali,
        aliquote_dettaglio,
    })
}

#[cfg(test)]
mod tests {
    use super::*;


    #[tokio::test]
    async fn test_payment_schedule_and_iva_liquidation_flow() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = FileStore::open(temp_dir.path().join("eattura.json")).unwrap();

        // 1. Create client
        let mut tx = store.begin().await.unwrap();
        tx.insert("clients", serde_json::json!({
            "id": "cli-comune-01",
            "denominazione": "Comune di Milano",
            "id_codice": "01199250158",
            "id_paese": "IT",
            "indirizzo": "Piazza della Scala 2",
            "cap": "20121",
            "comune": "Milano",
            "nazione": "IT",
            "codice_destinatario": "M5UXCR1",
            "pec": "protocollo@postacert.comune.milano.it"
        })).await.unwrap();
        tx.commit().await.unwrap();

        // 2. Create invoice in May 2026 (Q2) with Split Payment
        let inv_req = crate::commands::invoices::CreateInvoiceRequest {
            numero: "101/2026".to_string(),
            data: "2026-05-15".to_string(),
            tipo_documento: "TD01".to_string(),
            divisa: "EUR".to_string(),
            cedente_id: "cli-comune-01".to_string(),
            cessionario_id: "cli-comune-01".to_string(),
            causale: vec!["Lavori manutenzione straordinaria".to_string()],
            linee: vec![crate::commands::invoices::CreateLineRequest {
                descrizione: "Opere edili".to_string(),
                quantita: Some(1.0),
                unita_misura: Some("NR".to_string()),
                prezzo_unitario: 1000.0,
                aliquota_iva: 22.0,
                natura: None,
            }],
            pagamenti: vec![crate::commands::invoices::CreatePaymentRequest {
                modalita_pagamento: "MP05".to_string(),
                importo_pagamento: 1000.0,
                data_scadenza_pagamento: Some("2026-06-30".to_string()),
                iban: None,
            }],
            is_pa: Some(true),
            is_split_payment: Some(true),
            esigibilita_iva: Some("S".to_string()),
            ..Default::default()
        };

        let inv = crate::commands::invoices::create_invoice_store(&store, inv_req).await.unwrap();

        // Change status to accepted to be part of schedule
        let mut tx2 = store.begin().await.unwrap();
        tx2.update("invoices", &inv.id, serde_json::json!({
            "id": inv.id,
            "numero": inv.numero,
            "data": inv.data,
            "tipo_documento": inv.tipo_documento,
            "divisa": inv.divisa,
            "stato": "accepted",
            "cedente_id": inv.cedente_id,
            "cessionario_id": inv.cessionario_id,
            "direction": "outbound",
            "xml_content": ""
        })).await.unwrap();
        tx2.commit().await.unwrap();

        // 3. Check schedule
        let schedule = get_payment_schedule_store(&store, None).await.unwrap();
        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule[0].importo_totale, 1000.0);
        assert!(!schedule[0].pagata);

        // 4. Register payment
        let updated = register_invoice_payment_store(&store, RegisterPaymentRequest {
            invoice_id: inv.id.clone(),
            importo: 1000.0,
            data_pagamento: "2026-06-20".to_string(),
            modalita: Some("MP05".to_string()),
            note: Some("Saldato con mandato PA".to_string()),
            salda_tutto: true,
        }).await.unwrap();

        assert!(updated.pagata);
        assert_eq!(updated.saldo_residuo, 0.0);
        assert_eq!(updated.stato_scadenza, "saldata");

        // 5. Test IVA Liquidation for Q2 2026
        let liq = calculate_iva_liquidation_store(&store, IvaLiquidationRequest {
            year: 2026,
            period: "Q2".to_string(),
            credito_precedente: 0.0,
            is_trimestrale: true,
        }).await.unwrap();

        assert_eq!(liq.year, 2026);
        assert_eq!(liq.period, "Q2");
        assert_eq!(liq.codice_tributo_f24, "6032");
        assert_eq!(liq.data_scadenza_versamento, "2026-08-20");
        assert_eq!(liq.iva_vendite_totale, 220.0);
        // Split payment IVA is stornata
        assert_eq!(liq.iva_vendite_split_payment, 220.0);
        assert_eq!(liq.iva_vendite_esigibile, 0.0);
        assert_eq!(liq.totale_da_versare_f24, 0.0);
    }

    #[test]
    fn test_ledger_serde_roundtrip() {
        let mut ledger = PaymentsLedger {
            version: 1,
            entries: HashMap::new(),
        };

        ledger.entries.insert(
            "inv-01".to_string(),
            InvoiceLedgerEntry {
                invoice_id: "inv-01".to_string(),
                saldo_residuo: 0.0,
                pagata: true,
                pagamenti: vec![PaymentRecord {
                    id: "pay-01".to_string(),
                    data_pagamento: "2026-05-10".to_string(),
                    importo: 1220.0,
                    modalita: Some("MP05".to_string()),
                    note: Some("Bonifico saldato".to_string()),
                }],
                note: Some("Cliente affidabile".to_string()),
            },
        );

        let json_str = serde_json::to_string(&ledger).unwrap();
        let recovered: PaymentsLedger = serde_json::from_str(&json_str).unwrap();
        assert_eq!(recovered.entries.len(), 1);
        let entry = recovered.entries.get("inv-01").unwrap();
        assert!(entry.pagata);
        assert_eq!(entry.saldo_residuo, 0.0);
        assert_eq!(entry.pagamenti[0].importo, 1220.0);
    }
}
