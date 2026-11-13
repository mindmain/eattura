# Local archive

`../storage.rs` owns transactions and the exclusive archive lock. `index.rs` builds the in-memory catalog and `digest.rs` computes canonical IDs. Publish revision files before replacing `index.json`. Preserve old revisions and recoverable data when a write or migration fails. Keep reference checks and fiscal deduplication in the store.

Index Italian tax identifier warnings from the archived XML. Older indexes derive the warnings on open, without rewriting invoice revisions.
