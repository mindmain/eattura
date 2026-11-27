import { describe, expect, it } from "vitest";
import { humanizeBackendError } from "./tauri.service";

describe("humanizeBackendError", () => {
  it("translates Tauri/Serde invalid args errors into clear Italian", () => {
    const msg = humanizeBackendError(
      "invalid args `invoice` for command `create_invoice`: unknown field `isAutofattura`"
    );
    expect(msg).toContain("Impossibile inviare i dati della fattura al backend locale");
  });

  it("translates duplicate invoice errors into Italian", () => {
    expect(humanizeBackendError(new Error("Duplicate invoices record"))).toContain(
      "Esiste già una fattura con lo stesso numero"
    );
  });

  it("translates line period date order errors with line number", () => {
    expect(
      humanizeBackendError("Line 2: dataFinePeriodo precedes dataInizioPeriodo (2026-01-01 < 2026-02-01)")
    ).toBe("Riga 2: la data di fine periodo non può precedere la data di inizio.");
  });

  it("preserves already-localized messages verbatim", () => {
    expect(humanizeBackendError("Compila i campi obbligatori della fattura.")).toBe(
      "Compila i campi obbligatori della fattura."
    );
  });
});
