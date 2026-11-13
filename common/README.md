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

#### SdI regression coverage

The [SdI tests](src/validation/rules/sdi_tests.rs) exercise the ordinary invoice model using
[Annex A v1.9.1](https://www.agenziaentrate.gov.it/portale/documents/d/guest/allegato-a-specifiche-tecniche-vers-1-9-1)
and the [SdI controls v2.0](https://www.fatturapa.gov.it/export/documenti/fatturapa/v1.4/Elenco-Controlli-versione-2.0.pdf).
The positive fixture passes validation before encoding and after decoding. Run the suite with
`cargo test --locked -p common validation::`.

| Scope | Codes |
|---|---|
| Generated XML attachment name and size | 00001, 00003 |
| Supported schema checks and format error count | 00200, 00201 |
| Local tax identifier structure and check digits | 00300–00306 |
| Recipient routing and PEC | 00313, 00330, 00427, 00428 |
| Invoice dates, batch duplicates and numbering | 00403, 00409, 00418, 00425 |
| VAT, nature, pension contributions, withholding, adjustments and summaries | 00400, 00401, 00411, 00413–00415, 00419–00424, 00429, 00430, 00437, 00438, 00443–00445 |
| Customer identity, document type and party combinations | 00417, 00471–00476 |

`validate_on` takes the receipt date explicitly. `validate` uses the current local date
as the estimated receipt date during preflight. The PEC flow checks the generated
filename and XML byte count before reserving an SMTP attempt.

SdI resolves name and invoice duplicates across submissions (00002, 00404), registry
and routing status (00300–00306, 00311, 00312, 00320–00327, 00398, 00399), and invalidated
declarations of intent (00477) using data available to SdI. Identity rules 00471/00472
can also require a registry lookup when the customer has only a fiscal code. The
ordinary unsigned XML model does not represent signed files or ZIP containers
(00100–00107), simplified invoices (00460), or TD29 in the pinned Protobuf contract.
The 00200 checks cover selected fields and are not a full XSD validation.

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
