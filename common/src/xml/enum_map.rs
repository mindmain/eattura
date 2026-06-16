//! Bidirectional mapping between proto enum integer values and SDI XML string representations.
//!
//! Each SDI enum has a string code defined in the proto `(sdi).value` option.
//! For example: `FormatoTrasmissione::FPA12` (proto value 10) maps to XML string `"FPA12"`.
//!
//! These mappings are used by the XML encoder and decoder to convert between
//! proto integer discriminants and the string codes required by the SDI XML format.

/// Generates a pair of bidirectional mapping functions for an SDI enum.
///
/// For a given enum, this macro produces:
/// - `{name}_to_sdi(i32) -> Option<&'static str>` — converts a proto integer value to its SDI string code
/// - `{name}_from_sdi(&str) -> Option<i32>` — parses an SDI string code back to the proto integer value
///
/// # Example
///
/// ```ignore
/// sdi_enum_map!(formato_trasmissione, FormatoTrasmissione, {
///     10 => "FPA12",
///     20 => "FPR12",
/// });
/// // Generates:
/// // pub fn formato_trasmissione_to_sdi(value: i32) -> Option<&'static str>
/// // pub fn formato_trasmissione_from_sdi(s: &str) -> Option<i32>
/// ```
macro_rules! sdi_enum_map {
    ($name:ident, $enum_name:ident, { $($proto_val:expr => $sdi_val:expr),* $(,)? }) => {
        ::paste::paste! {
            #[doc = concat!("Convert a `", stringify!($enum_name), "` proto integer value to its SDI XML string code.")]
            #[doc = ""]
            #[doc = "Returns `None` for unrecognized values (including 0 / UNSPECIFIED)."]
            pub fn [<$name _to_sdi>](value: i32) -> Option<&'static str> {
                match value {
                    $($proto_val => Some($sdi_val),)*
                    _ => None,
                }
            }

            #[doc = concat!("Parse an SDI XML string code into a `", stringify!($enum_name), "` proto integer value.")]
            #[doc = ""]
            #[doc = "Returns `None` if the string does not match any known SDI code for this enum."]
            pub fn [<$name _from_sdi>](s: &str) -> Option<i32> {
                match s {
                    $($sdi_val => Some($proto_val),)*
                    _ => None,
                }
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Transmission format
// ---------------------------------------------------------------------------

sdi_enum_map!(formato_trasmissione, FormatoTrasmissione, {
    10 => "FPA12",
    20 => "FPR12",
});

// ---------------------------------------------------------------------------
// Document type (19 active variants, TD07-TD15 not defined in spec)
// ---------------------------------------------------------------------------

sdi_enum_map!(tipo_documento, TipoDocumento, {
    10  => "TD01",
    20  => "TD02",
    30  => "TD03",
    40  => "TD04",
    50  => "TD05",
    60  => "TD06",
    160 => "TD16",
    170 => "TD17",
    180 => "TD18",
    190 => "TD19",
    200 => "TD20",
    210 => "TD21",
    220 => "TD22",
    230 => "TD23",
    240 => "TD24",
    250 => "TD25",
    260 => "TD26",
    270 => "TD27",
    280 => "TD28",
});

// ---------------------------------------------------------------------------
// Withholding tax type
// ---------------------------------------------------------------------------

sdi_enum_map!(tipo_ritenuta, TipoRitenuta, {
    10 => "RT01",
    20 => "RT02",
    30 => "RT03",
    40 => "RT04",
    50 => "RT05",
    60 => "RT06",
});

// ---------------------------------------------------------------------------
// Professional fund type (22 variants)
// ---------------------------------------------------------------------------

sdi_enum_map!(tipo_cassa, TipoCassa, {
    10  => "TC01",
    20  => "TC02",
    30  => "TC03",
    40  => "TC04",
    50  => "TC05",
    60  => "TC06",
    70  => "TC07",
    80  => "TC08",
    90  => "TC09",
    100 => "TC10",
    110 => "TC11",
    120 => "TC12",
    130 => "TC13",
    140 => "TC14",
    150 => "TC15",
    160 => "TC16",
    170 => "TC17",
    180 => "TC18",
    190 => "TC19",
    200 => "TC20",
    210 => "TC21",
    220 => "TC22",
});

// ---------------------------------------------------------------------------
// Tax regime (no RF03 in the spec)
// ---------------------------------------------------------------------------

sdi_enum_map!(regime_fiscale, RegimeFiscale, {
    10  => "RF01",
    20  => "RF02",
    40  => "RF04",
    50  => "RF05",
    60  => "RF06",
    70  => "RF07",
    80  => "RF08",
    90  => "RF09",
    100 => "RF10",
    110 => "RF11",
    120 => "RF12",
    130 => "RF13",
    140 => "RF14",
    150 => "RF15",
    160 => "RF16",
    170 => "RF17",
    180 => "RF18",
    190 => "RF19",
});

// ---------------------------------------------------------------------------
// VAT nature / exemption code (25 variants including sub-codes)
// ---------------------------------------------------------------------------

sdi_enum_map!(natura, Natura, {
    10  => "N1",
    30  => "N2",
    50  => "N2.1",
    70  => "N2.2",
    90  => "N3",
    110 => "N3.1",
    130 => "N3.2",
    150 => "N3.3",
    170 => "N3.4",
    190 => "N3.5",
    210 => "N3.6",
    230 => "N4",
    250 => "N5",
    270 => "N6",
    290 => "N6.1",
    310 => "N6.2",
    330 => "N6.3",
    350 => "N6.4",
    370 => "N6.5",
    390 => "N6.6",
    410 => "N6.7",
    430 => "N6.8",
    450 => "N6.9",
    470 => "N7",
});

// ---------------------------------------------------------------------------
// Payment conditions
// ---------------------------------------------------------------------------

sdi_enum_map!(condizioni_pagamento, CondizioniPagamento, {
    10 => "TP01",
    20 => "TP02",
    30 => "TP03",
});

// ---------------------------------------------------------------------------
// Payment method (23 variants)
// ---------------------------------------------------------------------------

sdi_enum_map!(modalita_pagamento, ModalitaPagamento, {
    10  => "MP01",
    20  => "MP02",
    30  => "MP03",
    40  => "MP04",
    50  => "MP05",
    60  => "MP06",
    70  => "MP07",
    80  => "MP08",
    90  => "MP09",
    100 => "MP10",
    110 => "MP11",
    120 => "MP12",
    130 => "MP13",
    140 => "MP14",
    150 => "MP15",
    160 => "MP16",
    170 => "MP17",
    180 => "MP18",
    190 => "MP19",
    200 => "MP20",
    210 => "MP21",
    220 => "MP22",
    230 => "MP23",
});

// ---------------------------------------------------------------------------
// VAT exigibility
// ---------------------------------------------------------------------------

sdi_enum_map!(esigibilita_iva, EsigibilitaIVA, {
    10 => "D",
    20 => "I",
    30 => "S",
});

// ---------------------------------------------------------------------------
// Payment reason code (28 variants, note: no F in the spec)
// ---------------------------------------------------------------------------

sdi_enum_map!(causale_pagamento, CausalePagamento, {
    10  => "A",
    20  => "B",
    30  => "C",
    40  => "D",
    50  => "E",
    60  => "G",
    70  => "H",
    80  => "I",
    90  => "L",
    100 => "M",
    110 => "N",
    120 => "O",
    130 => "P",
    140 => "Q",
    150 => "R",
    160 => "S",
    170 => "T",
    180 => "U",
    190 => "V",
    200 => "W",
    210 => "X",
    220 => "Y",
    230 => "Z",
    240 => "L1",
    250 => "M1",
    260 => "M2",
    270 => "O1",
    280 => "V1",
    290 => "ZO",
});

// ---------------------------------------------------------------------------
// Discount / surcharge type
// ---------------------------------------------------------------------------

sdi_enum_map!(tipo_sconto_maggiorazione, TipoScontoMaggiorazione, {
    10 => "SC",
    20 => "MG",
});

// ---------------------------------------------------------------------------
// Boolean-like enums (single "SI" value)
// ---------------------------------------------------------------------------

sdi_enum_map!(art73, Art73, {
    10 => "SI",
});

sdi_enum_map!(bollo_virtuale, BolloVirtuale, {
    10 => "SI",
});

sdi_enum_map!(ritenuta, Ritenuta, {
    10 => "SI",
});

// ---------------------------------------------------------------------------
// Issuing subject
// ---------------------------------------------------------------------------

sdi_enum_map!(soggetto_emittente, SoggettoEmittente, {
    10 => "CC",
    20 => "TZ",
});

// ---------------------------------------------------------------------------
// Sole shareholder status
// ---------------------------------------------------------------------------

sdi_enum_map!(socio_unico, SocioUnico, {
    10 => "SU",
    20 => "SM",
});

// ---------------------------------------------------------------------------
// Liquidation status
// ---------------------------------------------------------------------------

sdi_enum_map!(stato_liquidazione, StatoLiquidazione, {
    10 => "LS",
    20 => "LN",
});

// ---------------------------------------------------------------------------
// Line item supply type
// ---------------------------------------------------------------------------

sdi_enum_map!(tipo_cessione_prestazione, TipoCessionePrestazione, {
    10 => "SC",
    20 => "PR",
    30 => "AB",
    40 => "AC",
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: verify round-trip for every (proto_value, sdi_string) pair in an enum mapping.
    macro_rules! test_enum_roundtrip {
        ($name:ident, { $($proto_val:expr => $sdi_val:expr),* $(,)? }) => {
            ::paste::paste! {
                #[test]
                fn [<test_ $name _roundtrip>]() {
                    // Forward: proto -> sdi
                    $(assert_eq!([<$name _to_sdi>]($proto_val), Some($sdi_val),
                        concat!("to_sdi failed for proto value ", stringify!($proto_val)));)*

                    // Reverse: sdi -> proto
                    $(assert_eq!([<$name _from_sdi>]($sdi_val), Some($proto_val),
                        concat!("from_sdi failed for SDI value ", $sdi_val));)*

                    // UNSPECIFIED (0) returns None
                    assert_eq!([<$name _to_sdi>](0), None, "UNSPECIFIED (0) should return None");

                    // Unknown values return None
                    assert_eq!([<$name _to_sdi>](99999), None, "Unknown proto value should return None");
                    assert_eq!([<$name _from_sdi>]("INVALID"), None, "Unknown SDI string should return None");
                }
            }
        };
    }

    test_enum_roundtrip!(formato_trasmissione, {
        10 => "FPA12", 20 => "FPR12",
    });

    test_enum_roundtrip!(tipo_documento, {
        10 => "TD01", 20 => "TD02", 30 => "TD03", 40 => "TD04", 50 => "TD05", 60 => "TD06",
        160 => "TD16", 170 => "TD17", 180 => "TD18", 190 => "TD19", 200 => "TD20",
        210 => "TD21", 220 => "TD22", 230 => "TD23", 240 => "TD24", 250 => "TD25",
        260 => "TD26", 270 => "TD27", 280 => "TD28",
    });

    test_enum_roundtrip!(tipo_ritenuta, {
        10 => "RT01", 20 => "RT02", 30 => "RT03", 40 => "RT04", 50 => "RT05", 60 => "RT06",
    });

    test_enum_roundtrip!(tipo_cassa, {
        10 => "TC01", 20 => "TC02", 30 => "TC03", 40 => "TC04", 50 => "TC05", 60 => "TC06",
        70 => "TC07", 80 => "TC08", 90 => "TC09", 100 => "TC10", 110 => "TC11", 120 => "TC12",
        130 => "TC13", 140 => "TC14", 150 => "TC15", 160 => "TC16", 170 => "TC17", 180 => "TC18",
        190 => "TC19", 200 => "TC20", 210 => "TC21", 220 => "TC22",
    });

    test_enum_roundtrip!(regime_fiscale, {
        10 => "RF01", 20 => "RF02", 40 => "RF04", 50 => "RF05", 60 => "RF06", 70 => "RF07",
        80 => "RF08", 90 => "RF09", 100 => "RF10", 110 => "RF11", 120 => "RF12", 130 => "RF13",
        140 => "RF14", 150 => "RF15", 160 => "RF16", 170 => "RF17", 180 => "RF18", 190 => "RF19",
    });

    test_enum_roundtrip!(natura, {
        10 => "N1", 30 => "N2", 50 => "N2.1", 70 => "N2.2", 90 => "N3",
        110 => "N3.1", 130 => "N3.2", 150 => "N3.3", 170 => "N3.4", 190 => "N3.5", 210 => "N3.6",
        230 => "N4", 250 => "N5", 270 => "N6",
        290 => "N6.1", 310 => "N6.2", 330 => "N6.3", 350 => "N6.4", 370 => "N6.5",
        390 => "N6.6", 410 => "N6.7", 430 => "N6.8", 450 => "N6.9", 470 => "N7",
    });

    test_enum_roundtrip!(condizioni_pagamento, {
        10 => "TP01", 20 => "TP02", 30 => "TP03",
    });

    test_enum_roundtrip!(modalita_pagamento, {
        10 => "MP01", 20 => "MP02", 30 => "MP03", 40 => "MP04", 50 => "MP05", 60 => "MP06",
        70 => "MP07", 80 => "MP08", 90 => "MP09", 100 => "MP10", 110 => "MP11", 120 => "MP12",
        130 => "MP13", 140 => "MP14", 150 => "MP15", 160 => "MP16", 170 => "MP17", 180 => "MP18",
        190 => "MP19", 200 => "MP20", 210 => "MP21", 220 => "MP22", 230 => "MP23",
    });

    test_enum_roundtrip!(esigibilita_iva, {
        10 => "D", 20 => "I", 30 => "S",
    });

    test_enum_roundtrip!(causale_pagamento, {
        10 => "A", 20 => "B", 30 => "C", 40 => "D", 50 => "E", 60 => "G", 70 => "H",
        80 => "I", 90 => "L", 100 => "M", 110 => "N", 120 => "O", 130 => "P", 140 => "Q",
        150 => "R", 160 => "S", 170 => "T", 180 => "U", 190 => "V", 200 => "W", 210 => "X",
        220 => "Y", 230 => "Z", 240 => "L1", 250 => "M1", 260 => "M2", 270 => "O1",
        280 => "V1", 290 => "ZO",
    });

    test_enum_roundtrip!(tipo_sconto_maggiorazione, {
        10 => "SC", 20 => "MG",
    });

    test_enum_roundtrip!(art73, { 10 => "SI" });
    test_enum_roundtrip!(bollo_virtuale, { 10 => "SI" });
    test_enum_roundtrip!(ritenuta, { 10 => "SI" });

    test_enum_roundtrip!(soggetto_emittente, { 10 => "CC", 20 => "TZ" });
    test_enum_roundtrip!(socio_unico, { 10 => "SU", 20 => "SM" });
    test_enum_roundtrip!(stato_liquidazione, { 10 => "LS", 20 => "LN" });

    test_enum_roundtrip!(tipo_cessione_prestazione, {
        10 => "SC", 20 => "PR", 30 => "AB", 40 => "AC",
    });
}
