import { Injectable, inject } from "@angular/core";
import { invoke } from "@tauri-apps/api/core";
import { AppLogService } from "./app-log.service";

/** Maps low-level Rust/Tauri/Serde errors to clear, actionable Italian UI messages. */
export function humanizeBackendError(error: unknown): string {
  const raw =
    error instanceof Error
      ? error.message
      : typeof error === "string"
        ? error
        : "Si è verificato un errore imprevisto.";

  if (raw.includes("Duplicate invoices record") || raw.includes("Invoice with the same content already exists")) {
    return "Esiste già una fattura con lo stesso numero, anno ed emittente nell’archivio.";
  }
  if (raw.includes("Duplicate clients record") || raw.includes("Client with the same fiscal identity already exists")) {
    return "Esiste già un’anagrafica con questa Partita IVA o Codice Fiscale.";
  }
  if (raw.includes("Client is referenced by invoices")) {
    return "Impossibile eliminare l’anagrafica perché è collegata a una o più fatture.";
  }
  if (raw.includes("Cannot change the fiscal identity of a client referenced by invoices")) {
    return "Impossibile modificare Partita IVA o Paese di un’anagrafica già utilizzata in fattura.";
  }
  if (raw.includes("Missing reference: cedente_id")) {
    return "Seleziona un Cedente (fornitore) valido presente in anagrafica.";
  }
  if (raw.includes("Missing reference: cessionario_id")) {
    return "Seleziona un Cessionario (cliente) valido presente in anagrafica.";
  }
  if (raw.includes("Draft not found")) {
    return "Bozza non trovata: ricarica l’elenco delle fatture prima di continuare.";
  }
  if (raw.includes("Draft changed")) {
    return "La bozza è stata aggiornata esternamente: riaprila per caricare l’ultima versione.";
  }
  if (raw.includes("only draft invoices are editable")) {
    return "Solo le fatture in stato Bozza possono essere modificate.";
  }
  if (raw.includes("dataFinePeriodo precedes dataInizioPeriodo")) {
    const lineMatch = raw.match(/Line (\d+)/i);
    return lineMatch
      ? `Riga ${lineMatch[1]}: la data di fine periodo non può precedere la data di inizio.`
      : "La data di fine periodo di una riga non può precedere la data di inizio.";
  }
  if (raw.includes("must be a date in YYYY-MM-DD format")) {
    const lineMatch = raw.match(/Line (\d+)/i);
    return lineMatch
      ? `Riga ${lineMatch[1]}: controlla che le date del periodo siano valide.`
      : "Formato data non valido: seleziona una data valida dal calendario.";
  }
  if (raw.includes("Descrizione must contain 1 to 1000 characters")) {
    const lineMatch = raw.match(/line (\d+)/i);
    return lineMatch
      ? `Riga ${lineMatch[1]}: inserisci una descrizione compresa tra 1 e 1.000 caratteri.`
      : "Ogni riga della fattura deve avere una descrizione compresa tra 1 e 1.000 caratteri.";
  }
  if (raw.includes("Descrizione contains an unsupported character")) {
    const lineMatch = raw.match(/line (\d+)/i);
    return lineMatch
      ? `Riga ${lineMatch[1]}: la descrizione contiene caratteri speciali non ammessi da SdI. Usa apostrofi (') e trattini (-) semplici.`
      : "La descrizione contiene caratteri non ammessi dal tracciato SdI.";
  }
  const sdiCountMatch = raw.match(/Cannot validate invoice: (\d+) SDI validation error/i);
  if (sdiCountMatch) {
    return `Impossibile validare la fattura: risolvi prima i ${sdiCountMatch[1]} errori SdI segnalati.`;
  }
  if (/^(invalid args|unknown field|missing field|invalid type)/i.test(raw) || raw.includes("unknown field `") || raw.includes("invalid args `")) {
    return "Impossibile inviare i dati della fattura al backend locale: controlla i campi compilati e riprova.";
  }
  return raw;
}

/**
 * Generic wrapper around Tauri's invoke API.
 * Provides type-safe IPC calls to Rust backend commands.
 */
@Injectable({ providedIn: "root" })
export class TauriService {
  private readonly log = inject(AppLogService);
  /**
   * Invoke a Tauri command with optional arguments.
   * Automatically handles serialization/deserialization and error wrapping.
   */
  async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    try {
      const result = await invoke<T>(command, args);
      if (/^(create_|update_|delete_|set_|send_|import_|export_|register_|switch_|rebuild_|sync_pec$)/.test(command)) {
        void this.log.record("info", command, "Operation completed");
      }
      return result;
    } catch (error) {
      // Password commands must not include even their error details in diagnostics.
      void this.log.record("error", command, command === "set_pec_password" ? "Unable to save PEC credentials" : error);
      throw new Error(humanizeBackendError(error));
    }
  }
}
