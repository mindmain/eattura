use base64::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::TauriState;
use crate::commands::invoices::{
    InvoiceDetail, InvoiceSummary, build_fattura_from_store, export_invoice_xml_store,
    fetch_invoice_detail, list_invoices_store,
};
use crate::commands::settings::AppSettings;
use crate::storage::FileStore;

/// Filters for generating accountant and AgID preservation export packages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFilter {
    pub year: i32,
    /// "all", "q1", "q2", "q3", "q4", or 2-digit month "01".."12"
    pub period: String,
    /// "all", "sale", "purchase"
    pub direction: String,
    #[serde(default = "default_true")]
    pub include_pdf: bool,
    #[serde(default = "default_true")]
    pub include_sdi_receipts: bool,
}

fn default_true() -> bool {
    true
}

/// DTO returned when generating a zip export package.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResultDto {
    pub filename: String,
    pub zip_base64: String,
    pub invoice_count: usize,
    pub sales_count: usize,
    pub purchase_count: usize,
    pub total_amount: f64,
}

/// DTO returned when exporting only the Prima Nota CSV.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvExportResultDto {
    pub filename: String,
    pub csv_content: String,
    pub row_count: usize,
}

/// Result of verifying an AgID SInCRO preservation package.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgidPreservationVerificationResult {
    pub valid: bool,
    pub package_id: String,
    pub created_at: String,
    pub producer: String,
    pub algorithm: String,
    pub total_files: usize,
    pub verified_files: usize,
    pub errors: Vec<String>,
}

/// Check if an invoice summary matches the user-selected export filter.
pub fn matches_export_filter(inv: &InvoiceSummary, filter: &ExportFilter) -> bool {
    // Check direction
    if filter.direction != "all" && inv.direction != filter.direction {
        return false;
    }

    // Check date format: YYYY-MM-DD
    let parts: Vec<&str> = inv.data.split('-').collect();
    if parts.len() < 2 {
        return false;
    }
    let Ok(year) = parts[0].parse::<i32>() else {
        return false;
    };
    if year != filter.year {
        return false;
    }
    let Ok(month) = parts[1].parse::<u32>() else {
        return false;
    };

    match filter.period.to_lowercase().as_str() {
        "all" => true,
        "q1" => (1..=3).contains(&month),
        "q2" => (4..=6).contains(&month),
        "q3" => (7..=9).contains(&month),
        "q4" => (10..=12).contains(&month),
        p => {
            if let Ok(m) = p.parse::<u32>() {
                month == m
            } else {
                true
            }
        }
    }
}

/// Format numbers with two decimals and comma as decimal separator for Italian accounting software.
fn format_it_num(val: f64) -> String {
    format!("{:.2}", val).replace('.', ",")
}

/// Escape text for CSV files according to RFC 4180.
fn csv_escape(s: &str) -> String {
    if s.contains(';') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Compute SHA-256 hex digest of a byte slice.
fn compute_sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Load a lookup map `client_id -> (partita_iva, codice_fiscale)` from the store.
async fn load_client_tax_map(store: &FileStore) -> HashMap<String, (String, String)> {
    let mut map = HashMap::new();
    if let Ok(rows) = store
        .read("clients", "all", vec![], "")
        .await
        .fetch_all()
        .await
    {
        for row in rows {
            let id: String = row.get("id");
            let id_codice: String = row.get::<Option<String>>("id_codice").unwrap_or_default();
            let cf: String = row
                .get::<Option<String>>("codice_fiscale")
                .unwrap_or_default();
            map.insert(id, (id_codice, cf));
        }
    }
    map
}

/// Generate Prima Nota CSV content formatted for Zucchetti, TeamSystem, Datev, and Excel.
pub fn generate_prima_nota_csv(
    invoices: &[(InvoiceDetail, String)],
    client_tax_map: &HashMap<String, (String, String)>,
) -> String {
    let mut out = String::from("\u{FEFF}"); // UTF-8 BOM for automatic Excel recognition

    // Header row
    out.push_str(
        "TIPO_DOC;SEZIONALE;NUMERO_DOC;DATA_DOC;DIREZIONE;PARTITA_IVA;CODICE_FISCALE;RAGIONE_SOCIALE;\
        IMPONIBILE_TOTALE;IVA_4;IVA_5;IVA_10;IVA_22;NON_IMPONIBILE_ESENTE;NATURA_PREVALENTE;\
        CONTRIBUTO_CASSA;RITENUTA_ACCONTO;IMPOSTA_BOLLO;SPLIT_PAYMENT;TOTALE_LORDO;NETTO_A_PAGARE;\
        MODALITA_PAGAMENTO;DATA_SCADENZA;STATO_SDI;NOME_FILE_XML\n",
    );

    for (inv, direction) in invoices {
        let is_purchase = direction == "purchase";
        let sezionabile = if is_purchase { "A1" } else { "V1" };
        let direzione_str = if is_purchase { "RICEVUTA" } else { "EMESSA" };

        let (party, party_id) = if is_purchase {
            (&inv.cedente_denominazione, &inv.cedente_id)
        } else {
            (&inv.cessionario_denominazione, &inv.cessionario_id)
        };

        let mut imponibile_totale = 0.0;
        let mut iva_4 = 0.0;
        let mut iva_5 = 0.0;
        let mut iva_10 = 0.0;
        let mut iva_22 = 0.0;
        let mut non_imponibile = 0.0;
        let mut natura_counts = HashMap::new();

        for line in &inv.linee {
            let price = match line.quantita {
                Some(q) => line.prezzo_unitario * q,
                None => line.prezzo_unitario,
            };
            imponibile_totale += price;

            if (line.aliquota_iva - 4.0).abs() < 0.01 {
                iva_4 += price * 0.04;
            } else if (line.aliquota_iva - 5.0).abs() < 0.01 {
                iva_5 += price * 0.05;
            } else if (line.aliquota_iva - 10.0).abs() < 0.01 {
                iva_10 += price * 0.10;
            } else if (line.aliquota_iva - 22.0).abs() < 0.01 {
                iva_22 += price * 0.22;
            } else if line.aliquota_iva == 0.0 {
                non_imponibile += price;
                if let Some(nat) = &line.natura {
                    *natura_counts.entry(nat.clone()).or_insert(0) += 1;
                }
            }
        }

        let prevalente_natura = natura_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(nat, _)| nat)
            .unwrap_or_default();

        let cassa_amt: f64 = inv
            .dati_cassa_previdenziale
            .iter()
            .map(|c| c.importo_contributo_cassa)
            .sum();

        let ritenuta_amt: f64 = inv.dati_ritenuta.iter().map(|r| r.importo_ritenuta).sum();

        let bollo_amt = inv
            .dati_bollo
            .as_ref()
            .and_then(|b| b.importo_bollo)
            .unwrap_or_else(|| {
                if inv
                    .dati_bollo
                    .as_ref()
                    .map(|b| b.bollo_virtuale == "SI")
                    .unwrap_or(false)
                {
                    2.0
                } else {
                    0.0
                }
            });

        let is_split = inv.is_split_payment || inv.esigibilita_iva.as_deref() == Some("S");
        let split_payment_amt = if is_split {
            iva_4 + iva_5 + iva_10 + iva_22
        } else {
            0.0
        };

        let payment_mode = inv
            .pagamenti
            .first()
            .map(|p| p.modalita_pagamento.as_str())
            .unwrap_or("MP05");

        let due_date = inv
            .pagamenti
            .first()
            .and_then(|p| p.data_scadenza_pagamento.as_deref())
            .unwrap_or("");

        let (p_iva, cf) = client_tax_map
            .get(party_id)
            .map(|(piva, cf)| (piva.as_str(), cf.as_str()))
            .unwrap_or(("", ""));

        let clean_num = inv.numero.replace('/', "_");
        let xml_name = format!("{}_{}.xml", inv.tipo_documento, clean_num);
        let tot = inv.importo_totale.map(format_it_num).unwrap_or_default();
        let netto = inv
            .netto_a_pagare
            .or(inv.importo_totale)
            .map(format_it_num)
            .unwrap_or_default();

        out.push_str(&format!(
            "{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}\n",
            csv_escape(&inv.tipo_documento),
            sezionabile,
            csv_escape(&inv.numero),
            inv.data,
            direzione_str,
            csv_escape(p_iva),
            csv_escape(cf),
            csv_escape(party),
            format_it_num(imponibile_totale),
            format_it_num(iva_4),
            format_it_num(iva_5),
            format_it_num(iva_10),
            format_it_num(iva_22),
            format_it_num(non_imponibile),
            csv_escape(&prevalente_natura),
            format_it_num(cassa_amt),
            format_it_num(ritenuta_amt),
            format_it_num(bollo_amt),
            format_it_num(split_payment_amt),
            tot,
            netto,
            payment_mode,
            due_date,
            csv_escape(&inv.stato),
            xml_name
        ));
    }

    out
}

/// Helper to load producer settings for AgID package.
async fn load_producer_info(store: &FileStore) -> (String, String, String) {
    let default_val = (
        "AZIENDA UTENTE".to_string(),
        "00000000000".to_string(),
        "00000000000".to_string(),
    );
    let Ok(Some(row)) = store
        .read("settings", "settings", vec![], "")
        .await
        .fetch_optional()
        .await
    else {
        return default_val;
    };

    let json: String = row.get("value");
    let Ok(settings) = serde_json::from_str::<AppSettings>(&json) else {
        return default_val;
    };

    if let Some(company) = settings.company {
        let denom = company
            .denominazione
            .unwrap_or_else(|| "AZIENDA UTENTE".to_string());
        (
            denom,
            company
                .codice_fiscale
                .unwrap_or_else(|| company.id_codice.clone()),
            company.id_codice,
        )
    } else {
        default_val
    }
}

/// Query SDI notification XML receipts for an invoice.
async fn fetch_sdi_receipts_for_invoice(
    store: &FileStore,
    invoice_id: &str,
) -> Vec<(String, String)> {
    let Ok(rows) = store
        .read(
            "sdi_notifications",
            "invoice_children",
            vec![serde_json::json!(invoice_id)],
            "",
        )
        .await
        .fetch_all()
        .await
    else {
        return Vec::new();
    };

    rows.into_iter()
        .filter_map(|r| {
            let tipo: String = r.get("tipo");
            let raw_xml: String = r.get("raw_xml");
            if !raw_xml.is_empty() {
                Some((tipo, raw_xml))
            } else {
                None
            }
        })
        .collect()
}

/// Export "Pacco per il Commercialista" ZIP archive.
#[tauri::command]
pub async fn export_accountant_package(
    state: tauri::State<'_, TauriState>,
    filter: ExportFilter,
) -> Result<ExportResultDto, String> {
    export_accountant_package_store(&state.store, filter).await
}

pub async fn export_accountant_package_store(
    store: &FileStore,
    filter: ExportFilter,
) -> Result<ExportResultDto, String> {
    let all_summaries = list_invoices_store(store).await?;
    let matching: Vec<&InvoiceSummary> = all_summaries
        .iter()
        .filter(|s| matches_export_filter(s, &filter))
        .collect();
    let client_tax_map = load_client_tax_map(store).await;

    let mut sales_details: Vec<(InvoiceDetail, String)> = Vec::new();
    let mut purchase_details: Vec<(InvoiceDetail, String)> = Vec::new();
    let mut total_amount = 0.0;

    for s in &matching {
        total_amount += s.importo_totale.unwrap_or(0.0);
        let detail = fetch_invoice_detail(store, &s.id).await?;
        if s.direction == "purchase" {
            purchase_details.push((detail, s.direction.clone()));
        } else {
            sales_details.push((detail, s.direction.clone()));
        }
    }

    let period_slug = filter.period.to_uppercase();
    let root_folder = format!("Eattura_Export_{}_{}", filter.year, period_slug);
    let zip_filename = format!("{root_folder}.zip");

    let mut zip_buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut zip_buffer));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // 1. Process Sales (Fatture Emesse)
        for (detail, _) in &sales_details {
            let fattura_opt = build_fattura_from_store(store, &detail.id).await.ok();
            let base_name = if let Some(ref f) = fattura_opt {
                common::pec::sdi_filename(f)
            } else {
                format!(
                    "{}_{}.xml",
                    detail.tipo_documento,
                    detail.numero.replace('/', "_")
                )
            };
            let clean_base = base_name.strip_suffix(".xml").unwrap_or(&base_name);

            // Add XML
            let xml_content = export_invoice_xml_store(store, &detail.id).await?;
            let xml_path = format!("{root_folder}/01_Fatture_Emesse/{base_name}");
            zip.start_file(&xml_path, options)
                .map_err(|e| e.to_string())?;
            zip.write_all(xml_content.as_bytes())
                .map_err(|e| e.to_string())?;

            // Add PDF if requested
            if filter.include_pdf
                && let Some(ref f) = fattura_opt
                && let Ok(pdf_bytes) = common::pdf::generate_courtesy_pdf(f)
            {
                let pdf_path = format!("{root_folder}/01_Fatture_Emesse/{clean_base}.pdf");
                zip.start_file(&pdf_path, options)
                    .map_err(|e| e.to_string())?;
                zip.write_all(&pdf_bytes).map_err(|e| e.to_string())?;
            }

            // Add SDI Receipts if requested
            if filter.include_sdi_receipts {
                let receipts = fetch_sdi_receipts_for_invoice(store, &detail.id).await;
                for (tipo, raw_xml) in receipts {
                    let receipt_path =
                        format!("{root_folder}/01_Fatture_Emesse/{clean_base}_{tipo}_001.xml");
                    zip.start_file(&receipt_path, options)
                        .map_err(|e| e.to_string())?;
                    zip.write_all(raw_xml.as_bytes())
                        .map_err(|e| e.to_string())?;
                }
            }
        }

        // 2. Process Purchases (Fatture Ricevute)
        for (detail, _) in &purchase_details {
            let fattura_opt = build_fattura_from_store(store, &detail.id).await.ok();
            let base_name = if let Some(ref f) = fattura_opt {
                common::pec::sdi_filename(f)
            } else {
                format!(
                    "{}_{}.xml",
                    detail.tipo_documento,
                    detail.numero.replace('/', "_")
                )
            };
            let clean_base = base_name.strip_suffix(".xml").unwrap_or(&base_name);

            // Add XML
            let xml_content = export_invoice_xml_store(store, &detail.id).await?;
            let xml_path = format!("{root_folder}/02_Fatture_Ricevute/{base_name}");
            zip.start_file(&xml_path, options)
                .map_err(|e| e.to_string())?;
            zip.write_all(xml_content.as_bytes())
                .map_err(|e| e.to_string())?;

            // Add PDF if requested
            if filter.include_pdf
                && let Some(ref f) = fattura_opt
                && let Ok(pdf_bytes) = common::pdf::generate_courtesy_pdf(f)
            {
                let pdf_path = format!("{root_folder}/02_Fatture_Ricevute/{clean_base}.pdf");
                zip.start_file(&pdf_path, options)
                    .map_err(|e| e.to_string())?;
                zip.write_all(&pdf_bytes).map_err(|e| e.to_string())?;
            }
        }

        // 3. Add Prima Nota CSV spreadsheets
        if !sales_details.is_empty() {
            let csv = generate_prima_nota_csv(&sales_details, &client_tax_map);
            let path = format!("{root_folder}/Riepilogo_Fatture_Emesse.csv");
            zip.start_file(&path, options).map_err(|e| e.to_string())?;
            zip.write_all(csv.as_bytes()).map_err(|e| e.to_string())?;
        }

        if !purchase_details.is_empty() {
            let csv = generate_prima_nota_csv(&purchase_details, &client_tax_map);
            let path = format!("{root_folder}/Riepilogo_Fatture_Ricevute.csv");
            zip.start_file(&path, options).map_err(|e| e.to_string())?;
            zip.write_all(csv.as_bytes()).map_err(|e| e.to_string())?;
        }

        if !sales_details.is_empty() && !purchase_details.is_empty() {
            let mut combined = sales_details.clone();
            combined.extend(purchase_details.clone());
            let csv = generate_prima_nota_csv(&combined, &client_tax_map);
            let path = format!(
                "{root_folder}/Prima_Nota_Completa_{}_{}.csv",
                filter.year, period_slug
            );
            zip.start_file(&path, options).map_err(|e| e.to_string())?;
            zip.write_all(csv.as_bytes()).map_err(|e| e.to_string())?;
        }

        zip.finish().map_err(|e| e.to_string())?;
    }

    let zip_base64 = BASE64_STANDARD.encode(&zip_buffer);
    let total_amount_f64: f64 = total_amount;

    Ok(ExportResultDto {
        filename: zip_filename,
        zip_base64,
        invoice_count: matching.len(),
        sales_count: sales_details.len(),
        purchase_count: purchase_details.len(),
        total_amount: (total_amount_f64 * 100.0).round() / 100.0,
    })
}

/// Export only the Prima Nota CSV for direct import or inspection.
#[tauri::command]
pub async fn export_accountant_csv(
    state: tauri::State<'_, TauriState>,
    filter: ExportFilter,
) -> Result<CsvExportResultDto, String> {
    export_accountant_csv_store(&state.store, filter).await
}

pub async fn export_accountant_csv_store(
    store: &FileStore,
    filter: ExportFilter,
) -> Result<CsvExportResultDto, String> {
    let all_summaries = list_invoices_store(store).await?;
    let matching: Vec<&InvoiceSummary> = all_summaries
        .iter()
        .filter(|s| matches_export_filter(s, &filter))
        .collect();
    let client_tax_map = load_client_tax_map(store).await;

    let mut details = Vec::new();
    for s in &matching {
        let detail = fetch_invoice_detail(store, &s.id).await?;
        details.push((detail, s.direction.clone()));
    }

    let csv_content = generate_prima_nota_csv(&details, &client_tax_map);
    let period_slug = filter.period.to_uppercase();
    let filename = format!("Prima_Nota_{}_{}.csv", filter.year, period_slug);

    Ok(CsvExportResultDto {
        filename,
        csv_content,
        row_count: details.len(),
    })
}

/// Representation of a file in the AgID SInCRO index.
struct SincroFileEntry {
    rel_path: String,
    role: String,
    format_type: String,
    size_bytes: usize,
    sha256_hex: String,
}

/// Generate compliant `indice_conservazione.xml` following AgID Technical Guidelines (UNI 11386:2010 SInCRO).
fn build_agid_sincro_xml(
    package_uuid: &str,
    created_at_utc: &str,
    producer: &(String, String, String),
    documents: &[(InvoiceDetail, Vec<SincroFileEntry>)],
) -> String {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<IndiceConservazione xmlns=\"http://www.agid.gov.it/conservazione/sincro/1.0\" versione=\"1.0\">\n");
    xml.push_str(&format!(
        "  <IdentificativoPacchetto>urn:uuid:{package_uuid}</IdentificativoPacchetto>\n"
    ));
    xml.push_str(&format!(
        "  <DataCreazionePacchetto>{created_at_utc}</DataCreazionePacchetto>\n"
    ));
    xml.push_str("  <SoggettoProduttore>\n");
    xml.push_str(&format!(
        "    <Denominazione>{}</Denominazione>\n",
        escape_xml(&producer.0)
    ));
    xml.push_str(&format!(
        "    <CodiceFiscale>{}</CodiceFiscale>\n",
        escape_xml(&producer.1)
    ));
    xml.push_str(&format!(
        "    <PartitaIVA>{}</PartitaIVA>\n",
        escape_xml(&producer.2)
    ));
    xml.push_str("    <Software>Eattura v0.1.0 (Zero-DB AgID Compliant)</Software>\n");
    xml.push_str("  </SoggettoProduttore>\n");
    xml.push_str(
        "  <StandardRiferimento>UNI 11386:2010 (SInCRO) / AgID LL.GG. 2021</StandardRiferimento>\n",
    );
    xml.push_str("  <AlgoritmoImpronta>SHA-256</AlgoritmoImpronta>\n");
    xml.push_str(&format!(
        "  <NumeroDocumenti>{}</NumeroDocumenti>\n",
        documents.len()
    ));
    xml.push_str("  <Documenti>\n");

    for (inv, files) in documents {
        xml.push_str(&format!("    <Documento id=\"{}\">\n", inv.id));
        xml.push_str("      <TipologiaDocumentale>FatturaElettronica</TipologiaDocumentale>\n");
        xml.push_str("      <MetadatiFiscali>\n");
        xml.push_str(&format!(
            "        <TipoDocumento>{}</TipoDocumento>\n",
            inv.tipo_documento
        ));
        xml.push_str(&format!(
            "        <Numero>{}</Numero>\n",
            escape_xml(&inv.numero)
        ));
        xml.push_str(&format!("        <Data>{}</Data>\n", inv.data));
        xml.push_str(&format!(
            "        <Cedente>{}</Cedente>\n",
            escape_xml(&inv.cedente_denominazione)
        ));
        xml.push_str(&format!(
            "        <Cessionario>{}</Cessionario>\n",
            escape_xml(&inv.cessionario_denominazione)
        ));
        if let Some(tot) = inv.importo_totale {
            xml.push_str(&format!(
                "        <ImportoTotale>{:.2}</ImportoTotale>\n",
                tot
            ));
        }
        xml.push_str(&format!("        <StatoSDI>{}</StatoSDI>\n", inv.stato));
        xml.push_str("      </MetadatiFiscali>\n");
        xml.push_str("      <FileComponenti>\n");
        for f in files {
            xml.push_str(&format!("        <FileComponente ruolo=\"{}\">\n", f.role));
            xml.push_str(&format!("          <NomeFile>{}</NomeFile>\n", f.rel_path));
            xml.push_str(&format!("          <Formato>{}</Formato>\n", f.format_type));
            xml.push_str(&format!(
                "          <DimensioneByte>{}</DimensioneByte>\n",
                f.size_bytes
            ));
            xml.push_str(&format!(
                "          <Impronta alg=\"SHA-256\">{}</Impronta>\n",
                f.sha256_hex
            ));
            xml.push_str("        </FileComponente>\n");
        }
        xml.push_str("      </FileComponenti>\n");
        xml.push_str("    </Documento>\n");
    }

    xml.push_str("  </Documenti>\n");
    xml.push_str("</IndiceConservazione>\n");
    xml
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Generate AgID Submission Information Package (SIP / SInCRO) for legal 10-year digital preservation.
#[tauri::command]
pub async fn export_agid_preservation_package(
    state: tauri::State<'_, TauriState>,
    filter: ExportFilter,
) -> Result<ExportResultDto, String> {
    export_agid_preservation_package_store(&state.store, filter).await
}

pub async fn export_agid_preservation_package_store(
    store: &FileStore,
    filter: ExportFilter,
) -> Result<ExportResultDto, String> {
    let all_summaries = list_invoices_store(store).await?;
    let matching: Vec<&InvoiceSummary> = all_summaries
        .iter()
        .filter(|s| matches_export_filter(s, &filter))
        .collect();

    let mut sales_details: Vec<(InvoiceDetail, String)> = Vec::new();
    let mut purchase_details: Vec<(InvoiceDetail, String)> = Vec::new();
    let mut total_amount = 0.0;

    for s in &matching {
        total_amount += s.importo_totale.unwrap_or(0.0);
        let detail = fetch_invoice_detail(store, &s.id).await?;
        if s.direction == "purchase" {
            purchase_details.push((detail, s.direction.clone()));
        } else {
            sales_details.push((detail, s.direction.clone()));
        }
    }

    let producer = load_producer_info(store).await;
    let package_uuid = uuid::Uuid::new_v4().to_string();
    let now_utc = chrono::Utc::now().to_rfc3339();

    let period_slug = filter.period.to_uppercase();
    let root_folder = format!("Eattura_Conservazione_{}_{}", filter.year, period_slug);
    let zip_filename = format!("{root_folder}.zip");

    let mut staged_files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut sincro_documents: Vec<(InvoiceDetail, Vec<SincroFileEntry>)> = Vec::new();

    // Collect all documents & files
    let mut all_docs: Vec<(InvoiceDetail, String)> = sales_details.clone();
    all_docs.extend(purchase_details.clone());

    for (detail, direction) in &all_docs {
        let is_purchase = direction == "purchase";
        let sub_folder = if is_purchase {
            "02_Fatture_Ricevute"
        } else {
            "01_Fatture_Emesse"
        };
        let fattura_opt = build_fattura_from_store(store, &detail.id).await.ok();
        let base_name = if let Some(ref f) = fattura_opt {
            common::pec::sdi_filename(f)
        } else {
            format!(
                "{}_{}.xml",
                detail.tipo_documento,
                detail.numero.replace('/', "_")
            )
        };
        let clean_base = base_name.strip_suffix(".xml").unwrap_or(&base_name);

        let mut file_entries = Vec::new();

        // 1. Primary XML file
        let xml_str = export_invoice_xml_store(store, &detail.id).await?;
        let xml_bytes = xml_str.into_bytes();
        let xml_rel = format!("{sub_folder}/{base_name}");
        let xml_sha = compute_sha256_hex(&xml_bytes);
        file_entries.push(SincroFileEntry {
            rel_path: xml_rel.clone(),
            role: "DocumentoPrincipale".to_string(),
            format_type: "XML".to_string(),
            size_bytes: xml_bytes.len(),
            sha256_hex: xml_sha,
        });
        staged_files.push((format!("{root_folder}/{xml_rel}"), xml_bytes));

        // 2. Courtesy PDF
        if filter.include_pdf
            && let Some(ref f) = fattura_opt
            && let Ok(pdf_bytes) = common::pdf::generate_courtesy_pdf(f)
        {
            let pdf_rel = format!("{sub_folder}/{clean_base}.pdf");
            let pdf_sha = compute_sha256_hex(&pdf_bytes);
            file_entries.push(SincroFileEntry {
                rel_path: pdf_rel.clone(),
                role: "AllegatoPdf".to_string(),
                format_type: "PDF".to_string(),
                size_bytes: pdf_bytes.len(),
                sha256_hex: pdf_sha,
            });
            staged_files.push((format!("{root_folder}/{pdf_rel}"), pdf_bytes));
        }

        // 3. SDI Receipts
        if filter.include_sdi_receipts {
            let receipts = fetch_sdi_receipts_for_invoice(store, &detail.id).await;
            for (tipo, raw_xml) in receipts {
                let r_bytes = raw_xml.into_bytes();
                let r_rel = format!("{sub_folder}/{clean_base}_{tipo}_001.xml");
                let r_sha = compute_sha256_hex(&r_bytes);
                file_entries.push(SincroFileEntry {
                    rel_path: r_rel.clone(),
                    role: "RicevutaSDI".to_string(),
                    format_type: "XML".to_string(),
                    size_bytes: r_bytes.len(),
                    sha256_hex: r_sha,
                });
                staged_files.push((format!("{root_folder}/{r_rel}"), r_bytes));
            }
        }

        sincro_documents.push((detail.clone(), file_entries));
    }

    // Generate indice_conservazione.xml
    let indice_xml = build_agid_sincro_xml(&package_uuid, &now_utc, &producer, &sincro_documents);

    // Package everything into ZIP
    let mut zip_buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut zip_buffer));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // Put indice_conservazione.xml at the root of the archive
        let index_path = format!("{root_folder}/indice_conservazione.xml");
        zip.start_file(&index_path, options)
            .map_err(|e| e.to_string())?;
        zip.write_all(indice_xml.as_bytes())
            .map_err(|e| e.to_string())?;

        // Write all staged files
        for (path, bytes) in staged_files {
            zip.start_file(&path, options).map_err(|e| e.to_string())?;
            zip.write_all(&bytes).map_err(|e| e.to_string())?;
        }

        zip.finish().map_err(|e| e.to_string())?;
    }

    let zip_base64 = BASE64_STANDARD.encode(&zip_buffer);
    let total_amount_f64: f64 = total_amount;

    Ok(ExportResultDto {
        filename: zip_filename,
        zip_base64,
        invoice_count: matching.len(),
        sales_count: sales_details.len(),
        purchase_count: purchase_details.len(),
        total_amount: (total_amount_f64 * 100.0).round() / 100.0,
    })
}

/// Verify integrity of an AgID preservation package (SIP / SInCRO) against `indice_conservazione.xml`.
#[tauri::command]
pub async fn verify_agid_preservation_package(
    zip_base64: String,
) -> Result<AgidPreservationVerificationResult, String> {
    let zip_bytes = BASE64_STANDARD
        .decode(&zip_base64)
        .map_err(|e| format!("Invalid base64 archive: {e}"))?;

    let cursor = Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| format!("Corrupt ZIP archive: {e}"))?;

    // Find indice_conservazione.xml
    let mut index_content = None;
    let mut root_prefix = String::new();

    for i in 0..archive.len() {
        let (name, is_index) = {
            let file = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = file.name().map_err(|e| e.to_string())?.to_string();
            let is_index = name.ends_with("indice_conservazione.xml");
            (name, is_index)
        };
        if is_index {
            let mut content = String::new();
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            file.read_to_string(&mut content)
                .map_err(|e| e.to_string())?;
            if let Some(pos) = name.rfind('/') {
                root_prefix = name[..=pos].to_string();
            }
            index_content = Some(content);
            break;
        }
    }

    let Some(index_xml) = index_content else {
        return Err(
            "File 'indice_conservazione.xml' non trovato all'interno del pacchetto".to_string(),
        );
    };

    // Parse essential tags from XML
    let package_id = extract_xml_tag(&index_xml, "IdentificativoPacchetto")
        .unwrap_or_else(|| "SCONOSCIUTO".to_string());
    let created_at = extract_xml_tag(&index_xml, "DataCreazionePacchetto")
        .unwrap_or_else(|| "SCONOSCIUTO".to_string());
    let producer =
        extract_xml_tag(&index_xml, "Denominazione").unwrap_or_else(|| "SCONOSCIUTO".to_string());
    let algorithm =
        extract_xml_tag(&index_xml, "AlgoritmoImpronta").unwrap_or_else(|| "SHA-256".to_string());

    let mut errors = Vec::new();
    let mut verified_files = 0;

    // Parse every <FileComponente>
    let components = extract_file_components(&index_xml);
    let total_files = components.len();

    for comp in &components {
        let full_zip_path = format!("{root_prefix}{}", comp.rel_path);
        match archive.by_name(&full_zip_path) {
            Ok(mut zip_file) => {
                let mut data = Vec::new();
                if let Err(e) = zip_file.read_to_end(&mut data) {
                    errors.push(format!("Errore lettura file {}: {e}", comp.rel_path));
                    continue;
                }

                if data.len() != comp.expected_size {
                    errors.push(format!(
                        "Dimensione errata per {}: attesi {} byte, trovati {} byte",
                        comp.rel_path,
                        comp.expected_size,
                        data.len()
                    ));
                    continue;
                }

                let actual_hash = compute_sha256_hex(&data);
                if actual_hash != comp.expected_sha256.to_lowercase() {
                    errors.push(format!(
                        "Impronta SHA-256 errata per {}: attesa {}, calcolata {}",
                        comp.rel_path, comp.expected_sha256, actual_hash
                    ));
                    continue;
                }

                verified_files += 1;
            }
            Err(_) => {
                errors.push(format!(
                    "File dichiarato nell'indice ma mancante nel pacchetto: {}",
                    comp.rel_path
                ));
            }
        }
    }

    Ok(AgidPreservationVerificationResult {
        valid: errors.is_empty(),
        package_id,
        created_at,
        producer,
        algorithm,
        total_files,
        verified_files,
        errors,
    })
}

struct ParsedComponent {
    rel_path: String,
    expected_size: usize,
    expected_sha256: String,
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let start_needle = format!("<{tag}");
    let end_tag = format!("</{tag}>");
    let start_idx = xml.find(&start_needle)?;
    let after_start = &xml[start_idx..];
    let tag_open_close = after_start.find('>')?;
    let content_start = start_idx + tag_open_close + 1;
    let end = xml[content_start..].find(&end_tag)? + content_start;
    Some(xml[content_start..end].trim().to_string())
}

fn extract_file_components(xml: &str) -> Vec<ParsedComponent> {
    let mut out = Vec::new();
    let start_needle = "<FileComponente";
    let end_needle = "</FileComponente>";

    let mut cursor = 0;
    while let Some(s) = xml[cursor..].find(start_needle) {
        let block_start = cursor + s;
        let Some(e) = xml[block_start..].find(end_needle) else {
            break;
        };
        let block_end = block_start + e + end_needle.len();
        let block = &xml[block_start..block_end];
        cursor = block_end;

        let rel_path = extract_xml_tag(block, "NomeFile").unwrap_or_default();
        let size_str = extract_xml_tag(block, "DimensioneByte").unwrap_or_default();
        let sha = extract_xml_tag(block, "Impronta").unwrap_or_default();

        if !rel_path.is_empty() && !sha.is_empty() {
            let expected_size = size_str.parse::<usize>().unwrap_or(0);
            out.push(ParsedComponent {
                rel_path,
                expected_size,
                expected_sha256: sha,
            });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_export_accountant_and_agid_lifecycle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = FileStore::open(temp_dir.path().join("eattura.json")).unwrap();

        // 1. Create client & settings
        let client_id = uuid::Uuid::new_v4().to_string();
        store
            .insert(
                "clients",
                serde_json::json!({
                    "id": &client_id,
                    "denominazione": "Azienda Committente SpA",
                    "id_paese": "IT",
                    "id_codice": "01234567890",
                    "codice_fiscale": "01234567890",
                    "codice_destinatario": "SUBM70N",
                    "regime_fiscale": "RF01",
                    "comune": "Milano",
                    "indirizzo": "Via Roma 1",
                    "cap": "20100",
                    "nazione": "IT",
                }),
            )
            .await
            .unwrap();

        // 2. Create invoice
        let inv_req = crate::commands::invoices::CreateInvoiceRequest {
            numero: "2026/001".to_string(),
            data: "2026-02-15".to_string(),
            tipo_documento: "TD01".to_string(),
            divisa: "EUR".to_string(),
            cedente_id: client_id.clone(),
            cessionario_id: client_id.clone(),
            causale: vec!["Consulenza software Q1 2026".to_string()],
            linee: vec![crate::commands::invoices::CreateLineRequest {
                descrizione: "Sviluppo backend Rust".to_string(),
                quantita: Some(10.0),
                unita_misura: Some("ORE".to_string()),
                prezzo_unitario: 100.0,
                aliquota_iva: 22.0,
                natura: None,
            }],
            pagamenti: vec![crate::commands::invoices::CreatePaymentRequest {
                modalita_pagamento: "MP05".to_string(),
                importo_pagamento: 1220.0,
                data_scadenza_pagamento: Some("2026-03-31".to_string()),
                iban: Some("IT60X0542811101000000123456".to_string()),
            }],
            dati_cassa_previdenziale: None,
            dati_ritenuta: None,
            dati_bollo: None,
            dati_fatture_collegate: None,
            soggetto_emittente: None,
            is_pa: Some(false),
            is_split_payment: Some(false),
            esigibilita_iva: None,
            dati_ordine_acquisto: None,
            dati_contratto: None,
            dati_convenzione: None,
            ..Default::default()
        };

        let inv_detail = crate::commands::invoices::create_invoice_store(&store, inv_req)
            .await
            .unwrap();
        assert_eq!(inv_detail.numero, "2026/001");

        // 3. Test Accountant CSV export
        let filter = ExportFilter {
            year: 2026,
            period: "q1".to_string(),
            direction: "all".to_string(),
            include_pdf: true,
            include_sdi_receipts: true,
        };

        let csv_res = export_accountant_csv_store(&store, filter.clone())
            .await
            .unwrap();
        assert_eq!(csv_res.row_count, 1);
        assert!(csv_res.csv_content.contains("2026/001"));
        assert!(csv_res.csv_content.contains("01234567890")); // P.IVA / CF populated
        assert!(csv_res.csv_content.contains("1000,00")); // imponibile
        assert!(csv_res.csv_content.contains("220,00")); // iva 22%
        assert!(csv_res.csv_content.contains("1220,00")); // totale lordo

        // 4. Test Accountant ZIP package
        let pkg_res = export_accountant_package_store(&store, filter.clone())
            .await
            .unwrap();
        assert_eq!(pkg_res.invoice_count, 1);
        assert!(pkg_res.zip_base64.len() > 100);

        // Verify content inside ZIP
        let zip_bytes = BASE64_STANDARD.decode(&pkg_res.zip_base64).unwrap();
        let mut archive = ZipArchive::new(Cursor::new(zip_bytes)).unwrap();
        let mut file_names: Vec<String> = Vec::new();
        for i in 0..archive.len() {
            let file = archive.by_index(i).unwrap();
            let name = file.name().map_err(|e| e.to_string()).unwrap().to_string();
            file_names.push(name);
        }
        assert!(file_names.iter().any(|n: &String| n.ends_with(".xml")));
        assert!(file_names.iter().any(|n: &String| n.ends_with(".pdf")));
        assert!(
            file_names
                .iter()
                .any(|n: &String| n.ends_with("Riepilogo_Fatture_Emesse.csv"))
        );

        // 5. Test AgID Preservation Package (SIP / SInCRO)
        let agid_res = export_agid_preservation_package_store(&store, filter.clone())
            .await
            .unwrap();
        assert_eq!(agid_res.invoice_count, 1);

        // 6. Test AgID Verification
        let verify_res = verify_agid_preservation_package(agid_res.zip_base64)
            .await
            .unwrap();

        assert!(verify_res.valid);
        assert_eq!(verify_res.errors.len(), 0);
        assert!(verify_res.verified_files >= 2); // xml + pdf
        assert_eq!(verify_res.algorithm, "SHA-256");
    }
}
