import { describe, it, expect } from "vitest";
import { TipoDocumentoPipe } from "./tipo-documento.pipe";
import { InvoiceStatusPipe } from "./invoice-status.pipe";
import { AppCurrencyPipe } from "./app-currency.pipe";
import { ClientNamePipe } from "./client-name.pipe";
import { ModalitaPagamentoPipe } from "./modalita-pagamento.pipe";
import { CondizioniPagamentoPipe } from "./condizioni-pagamento.pipe";

describe("Shared Pure Pipes", () => {
  describe("TipoDocumentoPipe", () => {
    const pipe = new TipoDocumentoPipe();

    it("ritorna la descrizione per codici noti", () => {
      expect(pipe.transform("TD01")).toBe("Fattura");
      expect(pipe.transform("TD04")).toBe("Nota di credito");
    });

    it("ritorna il codice originale se sconosciuto", () => {
      expect(pipe.transform("TD99")).toBe("TD99");
    });
  });

  describe("InvoiceStatusPipe", () => {
    const pipe = new InvoiceStatusPipe();

    it("ritorna l'etichetta italiana per stati noti", () => {
      expect(pipe.transform("draft")).toBe("Bozza");
      expect(pipe.transform("validated")).toBe("Validata");
      expect(pipe.transform("sent")).toBe("Inviata");
      expect(pipe.transform("delivered")).toBe("Consegnata");
      expect(pipe.transform("accepted")).toBe("Accettata");
      expect(pipe.transform("rejected")).toBe("Rifiutata");
    });

    it("ritorna il valore originale se sconosciuto", () => {
      expect(pipe.transform("unknown")).toBe("unknown");
    });
  });

  describe("AppCurrencyPipe", () => {
    const pipe = new AppCurrencyPipe();

    it("formatta importi numerici in valuta EUR", () => {
      const formatted = pipe.transform(1234.5);
      expect(formatted).toContain("1234,50");
      expect(formatted).toContain("€");
    });

    it("gestisce 0, null e undefined senza crash", () => {
      expect(pipe.transform(0)).toContain("0,00");
      expect(pipe.transform(null)).toBe("-");
      expect(pipe.transform(undefined)).toBe("-");
    });
  });

  describe("ClientNamePipe", () => {
    const pipe = new ClientNamePipe();

    it("restituisce la denominazione se presente", () => {
      expect(
        pipe.transform({
          id: "1",
          denominazione: "Acme SpA",
          nome: "Mario",
          cognome: "Rossi",
          idPaese: "IT",
          idCodice: "123",
          comune: "Roma",
        })
      ).toBe("Acme SpA");
    });

    it("restituisce nome e cognome per le persone fisiche", () => {
      expect(
        pipe.transform({
          id: "2",
          denominazione: null,
          nome: "Mario",
          cognome: "Rossi",
          idPaese: "IT",
          idCodice: "123",
          comune: "Roma",
        })
      ).toBe("Mario Rossi");
    });
  });

  describe("ModalitaPagamentoPipe & CondizioniPagamentoPipe", () => {
    const modalitaPipe = new ModalitaPagamentoPipe();
    const condizioniPipe = new CondizioniPagamentoPipe();

    it("traduce codici di pagamento", () => {
      expect(modalitaPipe.transform("MP05")).toContain("Bonifico");
      expect(condizioniPipe.transform("TP02")).toContain("completo");
    });
  });
});
