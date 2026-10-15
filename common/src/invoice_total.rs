use crate::sdi::v1::FatturaElettronicaBody;

/// Infer a gross document amount when the XML omits ImportoTotaleDocumento.
/// The VAT summaries include line amounts, taxable pension contributions,
/// and VAT. A declared stamp duty is not necessarily charged to the buyer.
pub fn inferred_gross_total(body: &FatturaElettronicaBody) -> Option<f64> {
    let summaries = &body.dati_beni_servizi.as_ref()?.dati_riepilogo;
    if summaries.is_empty() {
        return None;
    }

    let total: f64 = summaries
        .iter()
        .map(|summary| summary.imponibile_importo + summary.imposta)
        .sum();
    total.is_finite().then_some((total * 100.0).round() / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdi::v1::{DatiBeniServizi, DatiRiepilogo};

    #[test]
    fn sums_taxable_amounts_and_vat_across_summaries() {
        let body = FatturaElettronicaBody {
            dati_beni_servizi: Some(DatiBeniServizi {
                dati_riepilogo: vec![
                    DatiRiepilogo {
                        imponibile_importo: 100.0,
                        imposta: 22.0,
                        ..Default::default()
                    },
                    DatiRiepilogo {
                        imponibile_importo: 50.0,
                        imposta: 0.0,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(inferred_gross_total(&body), Some(172.0));
    }

    #[test]
    fn missing_summaries_do_not_become_zero() {
        assert_eq!(
            inferred_gross_total(&FatturaElettronicaBody::default()),
            None
        );
    }
}
