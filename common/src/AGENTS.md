# Shared invoice logic

- [xml](xml/AGENTS.md) converts FatturaPA documents and [validation](validation/AGENTS.md) applies SdI rules.
- `invoice_status.rs` defines lifecycle transitions. `invoice_total.rs` infers missing totals from VAT summaries.
- `pec.rs`, `p7m.rs`, `pdf.rs` and `vies.rs` handle notifications, signed files, courtesy PDFs and VAT lookup data.
