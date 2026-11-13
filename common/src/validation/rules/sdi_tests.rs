use super::{tests::minimal_invoice, validate, validate_italian_party_tax_ids};
use crate::sdi::v1::*;

fn check(invoice: &FatturaElettronica, expected: &[&str]) {
    let result = validate(invoice);
    let mut codes: Vec<_> = result
        .errors
        .iter()
        .map(|error| error.code.as_str())
        .collect();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes, expected, "{:#?}", result.errors);
}

fn document(invoice: &mut FatturaElettronica) -> &mut DatiGeneraliDocumento {
    invoice.body[0]
        .dati_generali
        .as_mut()
        .unwrap()
        .dati_generali_documento
        .as_mut()
        .unwrap()
}

fn goods(invoice: &mut FatturaElettronica) -> &mut DatiBeniServizi {
    invoice.body[0].dati_beni_servizi.as_mut().unwrap()
}

fn transmission(invoice: &mut FatturaElettronica) -> &mut DatiTrasmissione {
    invoice
        .header
        .as_mut()
        .unwrap()
        .dati_trasmissione
        .as_mut()
        .unwrap()
}

fn supplier(invoice: &mut FatturaElettronica) -> &mut IdFiscale {
    invoice
        .header
        .as_mut()
        .unwrap()
        .cedente_prestatore
        .as_mut()
        .unwrap()
        .dati_anagrafici
        .as_mut()
        .unwrap()
        .id_fiscale_iva
        .as_mut()
        .unwrap()
}

fn customer(invoice: &mut FatturaElettronica) -> &mut DatiAnagraficiCessionario {
    invoice
        .header
        .as_mut()
        .unwrap()
        .cessionario_committente
        .as_mut()
        .unwrap()
        .dati_anagrafici
        .as_mut()
        .unwrap()
}

fn set_vat(invoice: &mut FatturaElettronica, rate: f64, nature: Option<i32>) {
    let goods = goods(invoice);
    goods.dettaglio_linee[0].aliquota_iva = rate;
    goods.dettaglio_linee[0].natura = nature;
    goods.dati_riepilogo[0].aliquota_iva = rate;
    goods.dati_riepilogo[0].natura = nature;
    goods.dati_riepilogo[0].imposta = rate;
}

#[test]
fn complete_invoice_passes_before_and_after_xml_roundtrip() {
    let mut invoice = minimal_invoice();
    let doc = document(&mut invoice);
    doc.numero = "2024/001".into();
    doc.importo_totale_documento = Some(219.60);
    doc.causale = vec!["Consulenza tecnica & assistenza".into()];
    let goods = goods(&mut invoice);
    let line = &mut goods.dettaglio_linee[0];
    line.quantita = Some(2.0);
    line.sconto_maggiorazione.push(ScontoMaggiorazione {
        tipo: 10,
        importo: Some(10.0),
        percentuale: Some(5.0),
    });
    line.prezzo_totale = 180.0;
    goods.dati_riepilogo[0].imponibile_importo = 180.0;
    goods.dati_riepilogo[0].imposta = 39.60;
    invoice.body[0].dati_pagamento.push(DatiPagamento {
        condizioni_pagamento: 20,
        dettaglio_pagamento: vec![DettaglioPagamento {
            modalita_pagamento: 50,
            data_scadenza_pagamento: Some("2024-02-15".into()),
            importo_pagamento: 219.60,
            iban: Some("IT60X0542811101000000123456".into()),
            ..Default::default()
        }],
    });

    check(&invoice, &[]);
    let xml = crate::xml::encode::encode(&invoice).unwrap();
    assert!(xml.contains("<PrezzoTotale>180.00"));
    let decoded = crate::xml::decode::decode(&xml).unwrap();
    check(&decoded, &[]);
}

#[test]
fn sdi_00313_foreign_routing_requires_a_foreign_vat_id() {
    let mut invoice = minimal_invoice();
    transmission(&mut invoice).codice_destinatario = "XXXXXXX".into();
    check(&invoice, &["00313"]);
    customer(&mut invoice)
        .id_fiscale_iva
        .as_mut()
        .unwrap()
        .id_paese = "FR".into();
    check(&invoice, &[]);
}

#[test]
fn sdi_00330_recipient_pec_must_not_be_an_sdi_mailbox() {
    let mut invoice = minimal_invoice();
    transmission(&mut invoice).pec_destinatario = Some("SDI01@PEC.FATTURAPA.IT".into());
    check(&invoice, &["00330"]);
    transmission(&mut invoice).pec_destinatario = Some("accounting@example.test".into());
    check(&invoice, &[]);
}

#[test]
fn sdi_00409_duplicate_document_in_the_same_batch() {
    let mut invoice = minimal_invoice();
    invoice.body.push(invoice.body[0].clone());
    check(&invoice, &["00409"]);
    invoice.body[1]
        .dati_generali
        .as_mut()
        .unwrap()
        .dati_generali_documento
        .as_mut()
        .unwrap()
        .tipo_documento = 40;
    check(&invoice, &[]);
}

#[test]
fn sdi_00411_and_00415_withholding_requires_document_withholding_data() {
    let mut invoice = minimal_invoice();
    goods(&mut invoice).dettaglio_linee[0].ritenuta = Some(10);
    check(&invoice, &["00411"]);
    document(&mut invoice)
        .dati_cassa_previdenziale
        .push(DatiCassaPrevidenziale {
            tipo_cassa: 40,
            al_cassa: 4.0,
            aliquota_iva: 22.0,
            ritenuta: Some(10),
            ..Default::default()
        });
    check(&invoice, &["00411", "00415"]);
    document(&mut invoice).dati_ritenuta.push(DatiRitenuta {
        tipo_ritenuta: 10,
        importo_ritenuta: 20.0,
        aliquota_ritenuta: 20.0,
        causale_pagamento: 10,
    });
    check(&invoice, &[]);
}

#[test]
fn sdi_00413_and_00414_apply_to_pension_contributions() {
    let mut invoice = minimal_invoice();
    set_vat(&mut invoice, 0.0, Some(10));
    document(&mut invoice)
        .dati_cassa_previdenziale
        .push(DatiCassaPrevidenziale {
            tipo_cassa: 40,
            al_cassa: 4.0,
            ..Default::default()
        });
    check(&invoice, &["00413"]);
    set_vat(&mut invoice, 22.0, None);
    let cassa = &mut document(&mut invoice).dati_cassa_previdenziale[0];
    cassa.aliquota_iva = 22.0;
    cassa.natura = Some(10);
    goods(&mut invoice).dati_riepilogo.push(DatiRiepilogo {
        natura: Some(10),
        ..Default::default()
    });
    check(&invoice, &["00414"]);
}

#[test]
fn sdi_00417_customer_requires_at_least_one_tax_identifier() {
    let mut invoice = minimal_invoice();
    customer(&mut invoice).id_fiscale_iva = None;
    check(&invoice, &["00417"]);
    customer(&mut invoice).codice_fiscale = Some("RSSMRA80A01H501U".into());
    check(&invoice, &[]);
}

#[test]
fn sdi_00418_linked_invoice_cannot_be_newer() {
    let mut invoice = minimal_invoice();
    invoice.body[0]
        .dati_generali
        .as_mut()
        .unwrap()
        .dati_fatture_collegate
        .push(DatiDocumentiCorrelati {
            id_documento: "INV-1".into(),
            data: Some("2024-01-16".into()),
            ..Default::default()
        });
    check(&invoice, &["00418"]);
    document(&mut invoice).data = "2024-01-16".into();
    check(&invoice, &[]);
}

#[test]
fn sdi_00419_counts_rates_and_00443_checks_their_values() {
    let mut invoice = minimal_invoice();
    document(&mut invoice)
        .dati_cassa_previdenziale
        .push(DatiCassaPrevidenziale {
            tipo_cassa: 40,
            al_cassa: 4.0,
            aliquota_iva: 10.0,
            ..Default::default()
        });
    check(&invoice, &["00419", "00443"]);
    goods(&mut invoice).dati_riepilogo.push(DatiRiepilogo {
        aliquota_iva: 5.0,
        ..Default::default()
    });
    check(&invoice, &["00443"]);
    goods(&mut invoice).dati_riepilogo[1].aliquota_iva = 10.0;
    check(&invoice, &[]);
}

#[test]
fn sdi_00420_applies_to_every_reverse_charge_nature() {
    for nature in [290, 310, 330, 350, 370, 390, 410, 430, 450] {
        let mut invoice = minimal_invoice();
        set_vat(&mut invoice, 0.0, Some(nature));
        goods(&mut invoice).dati_riepilogo[0].esigibilita_iva = Some(30);
        check(&invoice, &["00420"]);
    }
}

#[test]
fn sdi_00421_rounds_tax_before_applying_one_cent_tolerance() {
    let mut invoice = minimal_invoice();
    let goods = goods(&mut invoice);
    goods.dettaglio_linee[0].prezzo_unitario = 100.03;
    goods.dettaglio_linee[0].prezzo_totale = 100.03;
    goods.dati_riepilogo[0].imponibile_importo = 100.03;
    goods.dati_riepilogo[0].imposta = 22.02;
    check(&invoice, &[]);
    self::goods(&mut invoice).dati_riepilogo[0].imposta = 22.03;
    check(&invoice, &["00421"]);
}

#[test]
fn sdi_00422_sums_by_vat_rate_and_allows_one_euro() {
    let mut invoice = minimal_invoice();
    set_vat(&mut invoice, 0.0, Some(10));
    let goods = goods(&mut invoice);
    let mut second = goods.dettaglio_linee[0].clone();
    second.numero_linea = 2;
    second.natura = Some(70);
    goods.dettaglio_linee.push(second);
    goods.dati_riepilogo[0].imponibile_importo = 99.50;
    goods.dati_riepilogo.push(DatiRiepilogo {
        natura: Some(70),
        imponibile_importo: 101.75,
        arrotondamento: Some(0.25),
        ..Default::default()
    });
    check(&invoice, &[]);
    self::goods(&mut invoice).dati_riepilogo[1].imponibile_importo = 101.76;
    check(&invoice, &["00422"]);
}

#[test]
fn sdi_00423_allows_one_cent_on_a_line_total() {
    let mut invoice = minimal_invoice();
    goods(&mut invoice).dettaglio_linee[0].prezzo_totale = 99.99;
    check(&invoice, &[]);
    goods(&mut invoice).dettaglio_linee[0].prezzo_totale = 99.98;
    check(&invoice, &["00423"]);
}

#[test]
fn sdi_00424_rejects_fractional_vat_in_all_three_blocks() {
    let mut invoice = minimal_invoice();
    set_vat(&mut invoice, 0.22, None);
    document(&mut invoice)
        .dati_cassa_previdenziale
        .push(DatiCassaPrevidenziale {
            tipo_cassa: 40,
            al_cassa: 4.0,
            aliquota_iva: 0.22,
            ..Default::default()
        });
    let result = validate(&invoice);
    assert_eq!(
        result
            .errors
            .iter()
            .filter(|error| error.code == "00424")
            .count(),
        3,
        "{result:?}"
    );
}

#[test]
fn sdi_00425_requires_a_digit_in_the_document_number() {
    let mut invoice = minimal_invoice();
    document(&mut invoice).numero = "INV-A".into();
    check(&invoice, &["00425"]);
    document(&mut invoice).numero = "INV-1A".into();
    check(&invoice, &[]);
}

#[test]
fn sdi_00427_and_00428_transmission_format_must_match_code_and_version() {
    let mut invoice = minimal_invoice();
    transmission(&mut invoice).codice_destinatario = "ABC123".into();
    check(&invoice, &["00427"]);
    transmission(&mut invoice).formato_trasmissione = 10;
    check(&invoice, &["00428"]);
    invoice.versione = 10;
    check(&invoice, &[]);
}

#[test]
fn sdi_00429_and_00430_apply_to_summaries_and_td16_allows_taxable_nature() {
    let mut invoice = minimal_invoice();
    set_vat(&mut invoice, 0.0, Some(10));
    goods(&mut invoice).dati_riepilogo[0].natura = None;
    check(&invoice, &["00429", "00444"]);

    set_vat(&mut invoice, 22.0, Some(290));
    check(&invoice, &["00401", "00430"]);
    document(&mut invoice).tipo_documento = 160;
    check(&invoice, &[]);
}

#[test]
fn sdi_00437_and_00438_adjustments_require_an_amount_or_percentage() {
    let mut invoice = minimal_invoice();
    let adjustment = ScontoMaggiorazione {
        tipo: 10,
        ..Default::default()
    };
    document(&mut invoice).sconto_maggiorazione.push(adjustment);
    goods(&mut invoice).dettaglio_linee[0]
        .sconto_maggiorazione
        .push(adjustment);
    check(&invoice, &["00437", "00438"]);
}

#[test]
fn sdi_00444_matches_pension_nature_and_00445_rejects_generic_nature() {
    let mut invoice = minimal_invoice();
    set_vat(&mut invoice, 0.0, Some(10));
    document(&mut invoice)
        .dati_cassa_previdenziale
        .push(DatiCassaPrevidenziale {
            tipo_cassa: 40,
            al_cassa: 4.0,
            natura: Some(70),
            ..Default::default()
        });
    check(&invoice, &["00444"]);
    for nature in [30, 90, 270] {
        set_vat(&mut invoice, 0.0, Some(nature));
        document(&mut invoice).dati_cassa_previdenziale[0].natura = Some(nature);
        let result = validate(&invoice);
        assert_eq!(result.errors.len(), 3, "{result:?}");
        check(&invoice, &["00445"]);
    }
}

#[test]
fn sdi_00471_and_00472_check_the_parties_for_each_document_type() {
    for kind in [10, 20, 30, 60, 160, 170, 180, 190, 200, 240, 250, 280] {
        let mut invoice = minimal_invoice();
        document(&mut invoice).tipo_documento = kind;
        if matches!(kind, 170 | 180 | 190 | 280) {
            supplier(&mut invoice).id_paese = "FR".into();
        }
        let same_party = supplier(&mut invoice).clone();
        customer(&mut invoice).id_fiscale_iva = Some(same_party);
        let result = validate(&invoice);
        assert!(
            result.errors.iter().any(|error| error.code == "00471"),
            "TD {kind}: {result:?}"
        );
    }
    for kind in [210, 270] {
        let mut invoice = minimal_invoice();
        document(&mut invoice).tipo_documento = kind;
        check(&invoice, &["00472"]);
        let same_party = supplier(&mut invoice).clone();
        customer(&mut invoice).id_fiscale_iva = Some(same_party);
        check(&invoice, &[]);
    }
    let mut invoice = minimal_invoice();
    document(&mut invoice).tipo_documento = 200;
    check(&invoice, &[]);
}

#[test]
fn sdi_00473_foreign_purchase_requires_a_foreign_supplier() {
    for kind in [170, 180, 190, 280] {
        let mut invoice = minimal_invoice();
        document(&mut invoice).tipo_documento = kind;
        check(&invoice, &["00473"]);
        supplier(&mut invoice).id_paese = "SM".into();
        check(&invoice, &[]);
    }
}

#[test]
fn sdi_00474_td21_rejects_zero_vat() {
    let mut invoice = minimal_invoice();
    document(&mut invoice).tipo_documento = 210;
    let same_party = supplier(&mut invoice).clone();
    customer(&mut invoice).id_fiscale_iva = Some(same_party);
    set_vat(&mut invoice, 0.0, Some(10));
    check(&invoice, &["00474"]);
}

#[test]
fn sdi_00475_selected_document_types_require_customer_vat() {
    for kind in [160, 170, 180, 190, 200, 220, 230, 280] {
        let mut invoice = minimal_invoice();
        document(&mut invoice).tipo_documento = kind;
        if matches!(kind, 170 | 180 | 190 | 280) {
            supplier(&mut invoice).id_paese = "FR".into();
        }
        customer(&mut invoice).id_fiscale_iva = None;
        customer(&mut invoice).codice_fiscale = Some("RSSMRA80A01H501U".into());
        check(&invoice, &["00475"]);
    }
}

#[test]
fn sdi_00476_both_parties_cannot_have_foreign_vat() {
    let mut invoice = minimal_invoice();
    supplier(&mut invoice).id_paese = "DE".into();
    customer(&mut invoice)
        .id_fiscale_iva
        .as_mut()
        .unwrap()
        .id_paese = "FR".into();
    check(&invoice, &["00476"]);
}

#[test]
fn sdi_00403_receipt_date_is_inclusive() {
    let mut invoice = minimal_invoice();
    let receipt_date = chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    assert!(super::validate_on(&invoice, receipt_date).is_valid());
    document(&mut invoice).data = "2024-01-16".into();
    let result = super::validate_on(&invoice, receipt_date);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].code, "00403");
    assert_eq!(
        result.errors[0].element_path.as_deref(),
        Some("Body[1]/DatiGenerali/DatiGeneraliDocumento/Data")
    );
}

#[test]
fn sdi_00409_article_73_uses_the_full_date() {
    let mut invoice = minimal_invoice();
    document(&mut invoice).art73 = Some(10);
    invoice.body.push(invoice.body[0].clone());
    check(&invoice, &["00409"]);
    let second = invoice.body[1]
        .dati_generali
        .as_mut()
        .unwrap()
        .dati_generali_documento
        .as_mut()
        .unwrap();
    second.data = "2024-02-15".into();
    check(&invoice, &[]);
}

#[test]
fn sdi_00300_to_00306_identify_the_role_of_invalid_italian_tax_ids() {
    let mut invoice = minimal_invoice();
    transmission(&mut invoice)
        .id_trasmittente
        .as_mut()
        .unwrap()
        .id_codice = "01234567890".into();
    supplier(&mut invoice).id_codice = "01234567890".into();
    let supplier = invoice
        .header
        .as_mut()
        .unwrap()
        .cedente_prestatore
        .as_mut()
        .unwrap()
        .dati_anagrafici
        .as_mut()
        .unwrap();
    supplier.codice_fiscale = Some("RSSMRA80A01H501V".into());
    customer(&mut invoice)
        .id_fiscale_iva
        .as_mut()
        .unwrap()
        .id_codice = "09876543210".into();
    customer(&mut invoice).codice_fiscale = Some("RSSMRA80A01H501V".into());
    invoice.header.as_mut().unwrap().rappresentante_fiscale = Some(RappresentanteFiscale {
        dati_anagrafici: Some(DatiAnagraficiRappresentante {
            id_fiscale_iva: Some(IdFiscale {
                id_paese: "IT".into(),
                id_codice: "01234567890".into(),
            }),
            codice_fiscale: Some("RSSMRA80A01H501V".into()),
            ..Default::default()
        }),
    });
    check(
        &invoice,
        &[
            "00300", "00301", "00302", "00303", "00304", "00305", "00306",
        ],
    );
}

#[test]
fn italian_tax_checks_skip_foreign_identifiers_and_check_a_domestic_fiscal_code_without_vat() {
    let mut invoice = minimal_invoice();
    let buyer = customer(&mut invoice);
    buyer.codice_fiscale = Some("FOREIGN-ID".into());
    let vat = buyer.id_fiscale_iva.as_mut().unwrap();
    vat.id_paese = "DE".into();
    vat.id_codice = "FOREIGN-VAT".into();
    check(&invoice, &[]);
    assert!(validate_italian_party_tax_ids(invoice.header.as_ref().unwrap()).is_empty());

    let buyer = customer(&mut invoice);
    buyer.id_fiscale_iva = None;
    buyer.codice_fiscale = Some("RSSMRA80A01H501V".into());
    check(&invoice, &["00306"]);
    let issues = validate_italian_party_tax_ids(invoice.header.as_ref().unwrap());
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, "00306");
}

#[test]
fn sdi_00200_requires_the_invoice_header_and_body() {
    let mut invoice = minimal_invoice();
    invoice.header = None;
    check(&invoice, &["00200"]);
    let mut invoice = minimal_invoice();
    invoice.body.clear();
    check(&invoice, &["00200"]);
}

#[test]
fn optional_stamp_amount_can_be_omitted() {
    let mut invoice = minimal_invoice();
    document(&mut invoice).dati_bollo = Some(DatiBollo {
        bollo_virtuale: 10,
        importo_bollo: None,
    });
    check(&invoice, &[]);
}

#[test]
fn sdi_00201_reports_more_than_fifty_known_format_errors() {
    let mut invoice = minimal_invoice();
    let goods = goods(&mut invoice);
    let mut invalid_line = goods.dettaglio_linee[0].clone();
    invalid_line.descrizione = "Servizio — consulenza".into();
    goods.dettaglio_linee = (1..=50)
        .map(|number| DettaglioLinee {
            numero_linea: number,
            ..invalid_line.clone()
        })
        .collect();
    goods.dati_riepilogo[0].imponibile_importo = 5000.0;
    goods.dati_riepilogo[0].imposta = 1100.0;
    check(&invoice, &["00200"]);
    let goods = self::goods(&mut invoice);
    invalid_line.numero_linea = 51;
    goods.dettaglio_linee.push(invalid_line);
    goods.dati_riepilogo[0].imponibile_importo = 5100.0;
    goods.dati_riepilogo[0].imposta = 1122.0;
    check(&invoice, &["00200", "00201"]);
}

#[test]
fn sdi_00200_reports_the_path_to_an_invalid_header_address() {
    let mut invoice = minimal_invoice();
    invoice
        .header
        .as_mut()
        .unwrap()
        .cedente_prestatore
        .as_mut()
        .unwrap()
        .sede
        .as_mut()
        .unwrap()
        .cap = "123".into();
    let result = validate(&invoice);
    assert_eq!(result.errors.len(), 1, "{result:?}");
    assert_eq!(result.errors[0].code, "00200");
    assert_eq!(
        result.errors[0].element_path.as_deref(),
        Some("Header/CedentePrestatore/Sede/CAP")
    );
}
