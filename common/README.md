# common

Shared Rust library for the Eattura electronic invoice system. Provides XML codec, validation engine, and SDI (Sistema di Interscambio) protocol buffer types used by both the server (`server-eattura`) and the desktop app (`eattura`).

## Modules

### `sdi::v1` — Protocol Buffer Types

Auto-generated Rust types from the `.proto` definitions under `proto/eattura/sdi/v1/`. These represent the complete FatturaPA v1.2.2 data model:

- `FatturaElettronica` — Root invoice message containing header and body
- `FatturaElettronicaHeader` — Transmission data, supplier (cedente), and buyer (cessionario)
- `FatturaElettronicaBody` — Document details, line items, payment terms, attachments
- 18+ SDI enums (`FormatoTrasmissione`, `TipoDocumento`, `RegimeFiscale`, `Natura`, etc.)
- Common types (`IdFiscale`, `Anagrafica`, `Indirizzo`, `ScontoMaggiorazione`, etc.)

All types derive `serde::Serialize` and `serde::Deserialize` in addition to standard prost traits.

### `xml` — XML Codec

Bidirectional conversion between `FatturaElettronica` proto structs and SDI-compliant XML.

#### Encoding

```rust
use common::sdi::v1::FatturaElettronica;
use common::xml::encode;

let invoice: FatturaElettronica = /* build invoice */;
let xml_string: String = encode::encode(&invoice)?;

// Or write directly to a file/stream:
let file = std::fs::File::create("invoice.xml")?;
encode::encode_to_writer(&invoice, file)?;
```

The encoder produces XML conforming to the Agenzia delle Entrate FatturaPA v1.2.2 schema:
- PascalCase element names matching the official XSD
- `p:FatturaElettronica` root element with SDI namespace
- Enum fields emitted as SDI string codes (e.g. `"FPA12"`, `"TD01"`, `"N2.1"`)
- Optional fields omitted when `None`, repeated fields as sibling elements
- Attachment bytes base64-encoded

#### Decoding

```rust
use common::xml::decode;

let xml = std::fs::read_to_string("invoice.xml")?;
let invoice = decode::decode(&xml)?;

// Or from a reader:
let file = std::fs::File::open("invoice.xml")?;
let invoice = decode::decode_from_reader(file)?;
```

The decoder handles namespace prefixes, `oneof` disambiguation (company name vs. individual name), enum string-to-integer conversion, and base64 attachment decoding.

#### Enum Mapping (`xml::enum_map`)

Bidirectional mapping between proto integer values and SDI XML string codes for all 18+ enums. Each enum gets two functions:

- `{name}_to_sdi(i32) -> Option<&'static str>` — proto value to XML string
- `{name}_from_sdi(&str) -> Option<i32>` — XML string to proto value

Returns `None` for unrecognized values (including `0` / UNSPECIFIED).

### `validation` — SDI Validation Engine

Validates a `FatturaElettronica` against SDI compliance rules, returning structured error results with SDI error codes.

```rust
use common::validation::rules::validate;

let result = validate(&invoice);
if result.is_valid() {
    println!("Invoice is valid");
} else {
    for error in &result.errors {
        println!("{}", error); // e.g. "[00400] Natura required when AliquotaIVA is 0"
    }
}
```

#### Implemented SDI Rules

| Code  | Description |
|-------|-------------|
| 00400 | Natura required when AliquotaIVA = 0 |
| 00401 | Natura must not be present when AliquotaIVA > 0 |
| 00419 | DatiRiepilogo must exist for each AliquotaIVA in line items |
| 00421 | Imposta must equal AliquotaIVA × ImponibileImporto / 100 |
| 00422 | ImponibileImporto must equal sum of PrezzoTotale for matching lines |
| 00423 | PrezzoTotale must equal PrezzoUnitario × Quantita ± adjustments |
| 00443 | Line AliquotaIVA must have matching DatiRiepilogo |
| 00444 | Line Natura must have matching DatiRiepilogo |
| 00471 | Self-invoicing types require matching cedente/cessionario P.IVA |
| 00473 | Foreign acquisition types require foreign cedente |

#### Format Validators

- **Codice Fiscale**: 11 digits (company) or 16 alphanumeric (individual)
- **IdFiscale**: 2-letter country code + 1-28 character tax code
- **IBAN**: Standard format validation

### `error` — Error Types

- `CommonError` — Top-level error wrapping XML and validation errors
- `XmlError` — Serialization, deserialization, unknown enum, missing element, I/O, parser errors
- `ValidationResult` / `ValidationError` — Structured validation results with SDI codes and element paths

## Dependencies

| Crate | Purpose |
|-------|---------|
| `prost` | Protocol Buffer runtime |
| `quick-xml` | XML parsing and writing |
| `serde` | Serialization framework |
| `thiserror` | Error derive macros |
| `paste` | Macro identifier concatenation |
| `base64` | Attachment encoding/decoding |

## Build

The crate uses `prost-build` at compile time to generate Rust types from the proto files in `../proto/eattura/sdi/v1/`. The `build.rs` script handles proto compilation with vendored `protoc`.

```sh
cargo build -p common
cargo test -p common
```
