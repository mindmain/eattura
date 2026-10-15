use std::io::{BufWriter, Cursor};
use printpdf::*;
use thiserror::Error;
use crate::sdi::v1::*;

#[derive(Debug, Error)]
pub enum PdfError {
    #[error("PDF generation failed: {0}")]
    Generation(String),
    #[error("XML decode failed: {0}")]
    Xml(#[from] crate::xml::XmlError),
}

/// Helper function to convert top-down Y (in mm) to bottom-up PDF coordinates.
fn y(top_offset: f32) -> Mm {
    Mm(297.0 - top_offset)
}

/// Extract clean name from `Anagrafica`
fn get_anagrafica_name(a: &Anagrafica) -> String {
    if let Some(soggetto) = &a.soggetto {
        match soggetto {
            anagrafica::Soggetto::Denominazione(d) => d.clone(),
            anagrafica::Soggetto::PersonaFisica(pf) => format!("{} {}", pf.cognome, pf.nome).trim().to_string(),
        }
    } else {
        String::new()
    }
}

/// Sanitize text for standard PDF built-in fonts (ASCII / Latin-1 safe)
pub fn sanitize_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '€' => out.push_str("EUR "),
            'à' | 'á' => out.push_str("a'"),
            'è' | 'é' => out.push_str("e'"),
            'ì' | 'í' => out.push_str("i'"),
            'ò' | 'ó' => out.push_str("o'"),
            'ù' | 'ú' => out.push_str("u'"),
            'À' | 'Á' => out.push_str("A'"),
            'È' | 'É' => out.push_str("E'"),
            'Ì' | 'Í' => out.push_str("I'"),
            'Ò' | 'Ó' => out.push_str("O'"),
            'Ù' | 'Ú' => out.push_str("U'"),
            '°' => out.push_str(" deg"),
            c if c.is_ascii() && !c.is_control() => out.push(c),
            c if c.is_whitespace() => out.push(' '),
            _ => out.push(' '),
        }
    }
    out
}

/// Generate a courtesy PDF from a `FatturaElettronica`
pub fn generate_courtesy_pdf(invoice: &FatturaElettronica) -> Result<Vec<u8>, PdfError> {
    let (doc, page1, layer1) = PdfDocument::new(
        "Fattura Elettronica - Copia di Cortesia",
        Mm(210.0),
        Mm(297.0),
        "Main",
    );

    let font_regular = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| PdfError::Generation(e.to_string()))?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| PdfError::Generation(e.to_string()))?;

    let header = invoice.header.as_ref();
    let body = invoice.body.first();

    let layer = doc.get_page(page1).get_layer(layer1);

    // Document Meta
    let dati_gen = body.and_then(|b| b.dati_generali.as_ref()).and_then(|g| g.dati_generali_documento.as_ref());
    let tipo_doc_code = dati_gen.map(|d| d.tipo_documento).unwrap_or(10);
    let tipo_doc_str = crate::xml::enum_map::tipo_documento_to_sdi(tipo_doc_code).unwrap_or("TD01");
    let is_autofattura = matches!(tipo_doc_code, 160 | 170 | 180 | 190 | 200 | 210 | 270);

    // Title
    let title_text = if is_autofattura {
        format!("AUTOFATTURA / INTEGRAZIONE ({})", tipo_doc_str)
    } else {
        format!("FATTURA ELETTRONICA ({})", tipo_doc_str)
    };
    layer.use_text(&title_text, 16.0, Mm(20.0), y(25.0), &font_bold);
    layer.use_text(
        "Copia di cortesia - Documento privo di valenza fiscale ai sensi dell'art. 21 D.P.R. 633/72",
        7.5,
        Mm(20.0),
        y(30.0),
        &font_regular,
    );
    let doc_num = dati_gen.map(|d| d.numero.as_str()).unwrap_or("N/D");
    let doc_date = dati_gen.map(|d| d.data.as_str()).unwrap_or("N/D");
    let doc_divisa = dati_gen.map(|d| d.divisa.as_str()).unwrap_or("EUR");
    let doc_total_amount = dati_gen
        .and_then(|d| d.importo_totale_documento)
        .or_else(|| body.and_then(crate::invoice_total::inferred_gross_total));
    let doc_totale = doc_total_amount
        .map(|val| format!("{:.2}", val))
        .unwrap_or_else(|| "N/D".to_string());

    layer.use_text(&format!("Numero: {}", sanitize_text(doc_num)), 10.0, Mm(135.0), y(22.0), &font_bold);
    layer.use_text(&format!("Data: {}", sanitize_text(doc_date)), 10.0, Mm(135.0), y(27.0), &font_regular);
    layer.use_text(&format!("Valuta: {}", sanitize_text(doc_divisa)), 10.0, Mm(135.0), y(32.0), &font_regular);

    // Divider Line
    let div_1 = Line {
        points: vec![
            (Point::new(Mm(20.0), y(36.0)), false),
            (Point::new(Mm(190.0), y(36.0)), false),
        ],
        is_closed: false,
    };
    layer.add_line(div_1);

    // Cedente / Prestatore (Left)
    layer.use_text("CEDENTE / PRESTATORE", 9.0, Mm(20.0), y(42.0), &font_bold);
    let mut cur_y = 47.0;
    if let Some(h) = header {
        if let Some(ced) = &h.cedente_prestatore {
            if let Some(anag) = &ced.dati_anagrafici {
                if let Some(a) = &anag.anagrafica {
                    let full_name = get_anagrafica_name(a);
                    if !full_name.is_empty() {
                        layer.use_text(&sanitize_text(&full_name), 9.0, Mm(20.0), y(cur_y), &font_bold);
                        cur_y += 5.0;
                    }
                }
                if let Some(cf) = &anag.codice_fiscale {
                    layer.use_text(&format!("C.F.: {}", sanitize_text(cf)), 8.5, Mm(20.0), y(cur_y), &font_regular);
                    cur_y += 4.5;
                }
                if let Some(id_fisc) = &anag.id_fiscale_iva {
                    layer.use_text(&format!("P.IVA: {}{}", sanitize_text(&id_fisc.id_paese), sanitize_text(&id_fisc.id_codice)), 8.5, Mm(20.0), y(cur_y), &font_regular);
                    cur_y += 4.5;
                }
            }
            if let Some(sede) = &ced.sede {
                let addr = format!("{} - {} {} ({})", sede.indirizzo, sede.cap, sede.comune, sede.provincia.as_deref().unwrap_or(""));
                layer.use_text(&sanitize_text(&addr), 8.5, Mm(20.0), y(cur_y), &font_regular);
            }
        }
    }

    // Cessionario / Committente (Right)
    layer.use_text("CESSIONARIO / COMMITTENTE", 9.0, Mm(110.0), y(42.0), &font_bold);
    let mut cur_y_dest = 47.0;
    if let Some(h) = header {
        if let Some(cess) = &h.cessionario_committente {
            if let Some(anag) = &cess.dati_anagrafici {
                if let Some(a) = &anag.anagrafica {
                    let full_name = get_anagrafica_name(a);
                    if !full_name.is_empty() {
                        layer.use_text(&sanitize_text(&full_name), 9.0, Mm(110.0), y(cur_y_dest), &font_bold);
                        cur_y_dest += 5.0;
                    }
                }
                if let Some(cf) = &anag.codice_fiscale {
                    layer.use_text(&format!("C.F.: {}", sanitize_text(cf)), 8.5, Mm(110.0), y(cur_y_dest), &font_regular);
                    cur_y_dest += 4.5;
                }
                if let Some(id_fisc) = &anag.id_fiscale_iva {
                    layer.use_text(&format!("P.IVA: {}{}", sanitize_text(&id_fisc.id_paese), sanitize_text(&id_fisc.id_codice)), 8.5, Mm(110.0), y(cur_y_dest), &font_regular);
                    cur_y_dest += 4.5;
                }
            }
            if let Some(sede) = &cess.sede {
                let addr = format!("{} - {} {} ({})", sede.indirizzo, sede.cap, sede.comune, sede.provincia.as_deref().unwrap_or(""));
                layer.use_text(&sanitize_text(&addr), 8.5, Mm(110.0), y(cur_y_dest), &font_regular);
            }
        }
        if let Some(dt) = &h.dati_trasmissione {
            layer.use_text(&format!("Codice SDI: {}", sanitize_text(&dt.codice_destinatario)), 8.5, Mm(110.0), y(cur_y_dest + 4.5), &font_regular);
        }
    }

    // Table Header
    let table_top = 75.0;
    let div_table = Line {
        points: vec![
            (Point::new(Mm(20.0), y(table_top)), false),
            (Point::new(Mm(190.0), y(table_top)), false),
        ],
        is_closed: false,
    };
    layer.add_line(div_table);

    layer.use_text("#", 8.5, Mm(22.0), y(table_top + 4.5), &font_bold);
    layer.use_text("Descrizione", 8.5, Mm(30.0), y(table_top + 4.5), &font_bold);
    layer.use_text("Qta", 8.5, Mm(115.0), y(table_top + 4.5), &font_bold);
    layer.use_text("Prezzo Unit.", 8.5, Mm(130.0), y(table_top + 4.5), &font_bold);
    layer.use_text("IVA", 8.5, Mm(155.0), y(table_top + 4.5), &font_bold);
    layer.use_text("Totale", 8.5, Mm(172.0), y(table_top + 4.5), &font_bold);

    let div_table_bot = Line {
        points: vec![
            (Point::new(Mm(20.0), y(table_top + 7.0)), false),
            (Point::new(Mm(190.0), y(table_top + 7.0)), false),
        ],
        is_closed: false,
    };
    layer.add_line(div_table_bot);

    // Line items
    let mut row_y = table_top + 13.0;
    if let Some(b) = body {
        if let Some(dbs) = &b.dati_beni_servizi {
            for line in &dbs.dettaglio_linee {
                if row_y > 220.0 {
                    break;
                }
                layer.use_text(&line.numero_linea.to_string(), 8.0, Mm(22.0), y(row_y), &font_regular);

                let desc = sanitize_text(&line.descrizione);
                let truncated_desc = if desc.len() > 45 {
                    format!("{}...", &desc[..42])
                } else {
                    desc
                };
                layer.use_text(&truncated_desc, 8.0, Mm(30.0), y(row_y), &font_regular);

                let qta_str = line.quantita.map(|q| format!("{:.2}", q)).unwrap_or_else(|| "1.00".to_string());
                layer.use_text(&sanitize_text(&qta_str), 8.0, Mm(115.0), y(row_y), &font_regular);

                layer.use_text(&format!("{:.2}", line.prezzo_unitario), 8.0, Mm(130.0), y(row_y), &font_regular);

                let iva_str = format!("{:.0}%", line.aliquota_iva);
                layer.use_text(&iva_str, 8.0, Mm(155.0), y(row_y), &font_regular);

                layer.use_text(&format!("{:.2}", line.prezzo_totale), 8.0, Mm(172.0), y(row_y), &font_regular);

                row_y += 6.0;
            }
        }
    }

    // Totals Section
    let mut tot_ritenuta = 0.0;
    let mut tot_cassa = 0.0;
    let mut tot_split_iva = 0.0;
    let mut has_split_payment = false;

    if let Some(dg) = dati_gen {
        for rit in &dg.dati_ritenuta {
            tot_ritenuta += rit.importo_ritenuta;
        }
        for cassa in &dg.dati_cassa_previdenziale {
            tot_cassa += cassa.importo_contributo_cassa;
        }
    }

    if let Some(b) = body {
        if let Some(dbs) = &b.dati_beni_servizi {
            for riep in &dbs.dati_riepilogo {
                if riep.esigibilita_iva == Some(30) { // S = Split Payment
                    has_split_payment = true;
                    tot_split_iva += riep.imposta;
                }
            }
        }
    }

    let sum_top = row_y + 4.0;
    let div_sum = Line {
        points: vec![
            (Point::new(Mm(100.0), y(sum_top)), false),
            (Point::new(Mm(190.0), y(sum_top)), false),
        ],
        is_closed: false,
    };
    layer.add_line(div_sum);

    let mut cur_sum_y = sum_top + 6.0;
    if tot_cassa > 0.0 {
        layer.use_text("Cassa Previdenziale:", 8.0, Mm(102.0), y(cur_sum_y), &font_regular);
        layer.use_text(&format!("EUR {:.2}", tot_cassa), 8.0, Mm(165.0), y(cur_sum_y), &font_regular);
        cur_sum_y += 5.0;
    }

    let total_label = if dati_gen.and_then(|d| d.importo_totale_documento).is_some() {
        "TOTALE DOCUMENTO (Lordo):"
    } else {
        "TOTALE CALCOLATO (Lordo):"
    };
    layer.use_text(total_label, 9.0, Mm(102.0), y(cur_sum_y), &font_bold);
    layer.use_text(&format!("EUR {}", sanitize_text(&doc_totale)), 9.0, Mm(165.0), y(cur_sum_y), &font_bold);
    cur_sum_y += 5.0;

    if tot_split_iva > 0.0 {
        layer.use_text("Scissione pagamenti (IVA PA):", 8.0, Mm(102.0), y(cur_sum_y), &font_regular);
        layer.use_text(&format!("-EUR {:.2}", tot_split_iva), 8.0, Mm(165.0), y(cur_sum_y), &font_regular);
        cur_sum_y += 5.0;
    }

    if tot_ritenuta > 0.0 {
        layer.use_text("Ritenuta d'acconto:", 8.0, Mm(102.0), y(cur_sum_y), &font_regular);
        layer.use_text(&format!("-EUR {:.2}", tot_ritenuta), 8.0, Mm(165.0), y(cur_sum_y), &font_regular);
        cur_sum_y += 5.0;
    }

    if let Some(doc_num) = doc_total_amount.filter(|_| tot_ritenuta > 0.0 || tot_split_iva > 0.0) {
        let netto = (doc_num - tot_ritenuta - tot_split_iva).max(0.0);
        layer.use_text("NETTO A PAGARE:", 10.0, Mm(102.0), y(cur_sum_y), &font_bold);
        layer.use_text(&format!("EUR {:.2}", netto), 10.0, Mm(165.0), y(cur_sum_y), &font_bold);
    }

    // Mandatory Legal Notes (Forfettario, Bollo, Split payment, Causale)
    let notes_top = 238.0;
    layer.use_text("NOTE E DICHIARAZIONI FISCALI:", 8.0, Mm(20.0), y(notes_top), &font_bold);
    let mut notes_y = notes_top + 4.5;

    // Check for forfettario (RF19 = 190)
    let is_forfettario = header
        .and_then(|h| h.cedente_prestatore.as_ref())
        .and_then(|cp| cp.dati_anagrafici.as_ref())
        .map(|da| da.regime_fiscale == 190)
        .unwrap_or(false);

    if is_forfettario {
        layer.use_text(
            "- Operazione in regime forfettario ex art. 1 c. 54-89 L. 190/2014, non soggetta a IVA ex art. 1 c. 58 L. 190/2014",
            7.0,
            Mm(20.0),
            y(notes_y),
            &font_regular,
        );
        notes_y += 3.5;
        layer.use_text(
            "- Operazione non soggetta a ritenuta alla fonte a titolo di acconto ex art. 1 c. 67 L. 190/2014",
            7.0,
            Mm(20.0),
            y(notes_y),
            &font_regular,
        );
        notes_y += 3.5;
    }

    // Check for bollo
    if let Some(dg) = dati_gen {
        if let Some(bollo) = &dg.dati_bollo {
            if bollo.bollo_virtuale == 1 { // SI
                layer.use_text(
                    "- Imposta di bollo assolta in modo virtuale ai sensi del D.M. 17 giugno 2014 (EUR 2.00)",
                    7.0,
                    Mm(20.0),
                    y(notes_y),
                    &font_regular,
                );
                notes_y += 3.5;
            }
        }

        // Causale lines
        for c in &dg.causale {
            layer.use_text(
                &format!("- {}", sanitize_text(c)),
                7.0,
                Mm(20.0),
                y(notes_y),
                &font_regular,
            );
            notes_y += 3.5;
        }
    }

    if has_split_payment {
        layer.use_text(
            "- Operazione con scissione dei pagamenti (Split Payment) ex art. 17-ter D.P.R. 633/1972 (IVA a carico committente PA)",
            7.0,
            Mm(20.0),
            y(notes_y),
            &font_bold,
        );
        notes_y += 3.5;
    }

    // Dati Ordine di Acquisto / Contratto PA (CIG / CUP)
    if let Some(b) = body {
        if let Some(dg_block) = &b.dati_generali {
            for ord in &dg_block.dati_ordine_acquisto {
                let mut info = format!("- Ordine di Acquisto PA: N. {}", ord.id_documento);
                if let Some(d) = &ord.data {
                    info.push_str(&format!(" del {}", d));
                }
                if let Some(cig) = &ord.codice_cig {
                    info.push_str(&format!(" | CIG: {}", cig));
                }
                if let Some(cup) = &ord.codice_cup {
                    info.push_str(&format!(" | CUP: {}", cup));
                }
                layer.use_text(&sanitize_text(&info), 7.0, Mm(20.0), y(notes_y), &font_bold);
                notes_y += 3.5;
            }
            for con in &dg_block.dati_contratto {
                let mut info = format!("- Contratto PA: N. {}", con.id_documento);
                if let Some(d) = &con.data {
                    info.push_str(&format!(" del {}", d));
                }
                if let Some(cig) = &con.codice_cig {
                    info.push_str(&format!(" | CIG: {}", cig));
                }
                if let Some(cup) = &con.codice_cup {
                    info.push_str(&format!(" | CUP: {}", cup));
                }
                layer.use_text(&sanitize_text(&info), 7.0, Mm(20.0), y(notes_y), &font_bold);
                notes_y += 3.5;
            }
        }
    }

    // Dati Fatture Collegate (TD16, TD17, TD18, TD19)
    if let Some(b) = body {
        if let Some(dg_block) = &b.dati_generali {
            for fc in &dg_block.dati_fatture_collegate {
                let doc_ref = format!(
                    "- Fattura di riferimento collegata: n. {} del {}",
                    fc.id_documento,
                    fc.data.as_deref().unwrap_or("N/D")
                );
                layer.use_text(&sanitize_text(&doc_ref), 7.0, Mm(20.0), y(notes_y), &font_bold);
                notes_y += 3.5;
            }
        }
    }

    if is_autofattura {
        layer.use_text(
            "- Inversione contabile (Reverse Charge) ex art. 17 c. 2 D.P.R. 633/72 / D.L. 331/93 (IVA a debito e credito)",
            7.0,
            Mm(20.0),
            y(notes_y),
            &font_regular,
        );
        notes_y += 3.5;
    }

    // Generator note
    layer.use_text(
        "- Generato con Eattura - Architettura Sovrana Senza Database",
        6.5,
        Mm(20.0),
        y(notes_y),
        &font_regular,
    );

    let cursor = Cursor::new(Vec::new());
    let mut writer = BufWriter::new(cursor);
    doc.save(&mut writer)
        .map_err(|e| PdfError::Generation(e.to_string()))?;

    let inner = writer.into_inner().map_err(|e| PdfError::Generation(e.to_string()))?;
    Ok(inner.into_inner())
}

/// Generate a courtesy PDF directly from raw XML string
pub fn generate_pdf_from_xml(xml: &str) -> Result<Vec<u8>, PdfError> {
    let fattura = crate::xml::decode::decode(xml)?;
    generate_courtesy_pdf(&fattura)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_text() {
        assert_eq!(sanitize_text("Caffè € 1,50"), "Caffe' EUR  1,50");
        assert_eq!(sanitize_text("Società S.r.l."), "Societa' S.r.l.");
    }
}
