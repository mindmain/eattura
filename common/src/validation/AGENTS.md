# SdI validation

Put document and line rules in `rules.rs` with an error code and field path. Reuse the shared discount calculation for line totals. Run `cargo test --locked -p common validation::`.

Reuse the Italian tax identifier checks for archive warnings. These warnings read the identifiers from the archived XML header.
