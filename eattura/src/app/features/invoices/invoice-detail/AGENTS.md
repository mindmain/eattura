# Invoice review

Show actual XML destination, SdI filename (`<IdPaese><IdCodice>_<ProgressivoInvio>.xml` / `.pdf` for XML and courtesy PDF downloads) and line periods. Return a validated or `NS`-discarded invoice to draft before edits, keeping post-delivery PA refusals (`NE EC02`) distinct and locked. Keep PEC transmission manual and resolve pending attempts before allowing retries or status changes.
