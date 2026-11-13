import { describe, it, expect, beforeEach, vi } from "vitest";
import { TestBed } from "@angular/core/testing";
import { Router } from "@angular/router";
import { FormBuilder } from "@angular/forms";
import { DestroyRef, signal } from "@angular/core";
import { InvoiceFormComponent } from "./invoice-form.component";
import { InvoiceStore } from "../../../core/stores/invoice.store";
import { ClientStore } from "../../../core/stores/client.store";
import { SettingsStore } from "../../../core/stores/settings.store";
import { NotificationService } from "../../../core/services/notification.service";

describe("InvoiceFormComponent", () => {
  let component: InvoiceFormComponent;
  let routerMock: { navigate: ReturnType<typeof vi.fn> };
  let notifyMock: { success: ReturnType<typeof vi.fn>; error: ReturnType<typeof vi.fn> };
  let invoiceStoreMock: {
    getInvoice: ReturnType<typeof vi.fn>;
    createInvoice: ReturnType<typeof vi.fn>;
    updateInvoice: ReturnType<typeof vi.fn>;
    validateInvoice: ReturnType<typeof vi.fn>;
    nextInvoiceNumber: ReturnType<typeof vi.fn>;
  };
  let clientStoreMock: {
    loadAll: ReturnType<typeof vi.fn>;
    clients: ReturnType<typeof signal>;
    clientMap: ReturnType<typeof signal>;
  };
  let settingsStoreMock: {
    load: ReturnType<typeof vi.fn>;
    findCompanyClientId: ReturnType<typeof vi.fn>;
  };

  beforeEach(async () => {
    routerMock = { navigate: vi.fn() };
    notifyMock = { success: vi.fn(), error: vi.fn() };
    invoiceStoreMock = {
      getInvoice: vi.fn(),
      createInvoice: vi.fn().mockResolvedValue({ id: "new-inv-id" }),
      updateInvoice: vi.fn().mockResolvedValue(undefined),
      validateInvoice: vi.fn().mockResolvedValue({ valid: true, errors: [] }),
      nextInvoiceNumber: vi.fn().mockResolvedValue("1"),
    };
    clientStoreMock = {
      loadAll: vi.fn().mockResolvedValue([]),
      clients: signal([]),
      clientMap: signal(new Map()),
    };
    settingsStoreMock = {
      load: vi.fn().mockResolvedValue(null),
      findCompanyClientId: vi.fn().mockReturnValue(null),
    };

    TestBed.configureTestingModule({
      providers: [
        FormBuilder,
        DestroyRef,
        { provide: Router, useValue: routerMock },
        { provide: NotificationService, useValue: notifyMock },
        { provide: InvoiceStore, useValue: invoiceStoreMock },
        { provide: ClientStore, useValue: clientStoreMock },
        { provide: SettingsStore, useValue: settingsStoreMock },
      ],
    });

    component = TestBed.runInInjectionContext(() => new InvoiceFormComponent());
    await component.ngOnInit();
  });

  it("inizializza il form in modalità creazione con 1 riga vuota", () => {
    expect(component.linee.length).toBe(1);
    expect(component.isEditMode()).toBe(false);
  });

  it("addLine() e removeLine() modificano il numero di righe", () => {
    expect(component.linee.length).toBe(1);

    component.addLine();
    expect(component.linee.length).toBe(2);

    component.removeLine(0);
    expect(component.linee.length).toBe(1);
  });

  it("runningTotal() calcola reattivamente il totale in base a quantita e prezzo", () => {
    const firstLine = component.linee.at(0);
    firstLine.patchValue({
      quantita: 2,
      prezzoUnitario: 50,
    });

    expect(component.runningTotal()).toBe(100);

    component.addLine();
    const secondLine = component.linee.at(1);
    secondLine.patchValue({
      quantita: 3,
      prezzoUnitario: 20,
    });

    expect(component.runningTotal()).toBe(160);
  });

  it("calcola correttamente cassa previdenziale, ritenuta, bollo e netto a pagare", () => {
    // Riga 1000 EUR con IVA 22%
    component.linee.at(0).patchValue({
      quantita: 1,
      prezzoUnitario: 1000,
      aliquotaIva: 22,
    });

    // Attiva Cassa Inarcassa TC04 4% soggetta a IVA
    component.form.patchValue({
      applicaCassa: true,
      tipoCassa: "TC04",
      alCassa: 4.0,
      cassaSoggettaIva: true,
      // Attiva Ritenuta 20%
      applicaRitenuta: true,
      tipoRitenuta: "RT01",
      aliquotaRitenuta: 20.0,
      causalePagamentoRitenuta: "A",
    });

    const fin = component.financialSummary();
    expect(fin.imponibileLinee).toBe(1000);
    expect(fin.contributoCassa).toBe(40); // 4% di 1000
    // IVA: 22% su 1000 (220) + 22% su 40 (8.80) = 228.80
    expect(fin.totIva).toBe(228.80);
    // Lordo = 1000 + 40 + 228.80 = 1268.80
    expect(fin.totaleLordo).toBe(1268.80);
    // Ritenuta 20% su 1000 = 200
    expect(fin.importoRitenuta).toBe(200);
    // Netto a pagare = 1268.80 - 200 = 1068.80
    expect(fin.nettoAPagare).toBe(1068.80);

    // Test sync payment
    component.syncPaymentAmount();
    expect(component.form.get("importoPagamento")?.value).toBe(1068.80);
  });

  it("gestisce le automazioni del regime forfettario (RF19)", () => {
    component.linee.at(0).patchValue({
      quantita: 1,
      prezzoUnitario: 500,
      aliquotaIva: 22,
    });

    // Attiva regime forfettario
    component.toggleForfettario(true);

    expect(component.form.get("isForfettario")?.value).toBe(true);
    // IVA riga azzerata a 0% e natura N2.2
    expect(component.linee.at(0).get("aliquotaIva")?.value).toBe(0);
    expect(component.linee.at(0).get("natura")?.value).toBe("N2.2");
    // Ritenuta inibita
    expect(component.form.get("applicaRitenuta")?.value).toBe(false);
    // Bollo virtuale auto-attivato (> 77.47)
    expect(component.form.get("applicaBollo")?.value).toBe(true);
    // Causale arricchita con riferimento normativo L. 190/2014
    expect(component.form.get("causale")?.value).toContain("Legge n. 190/2014");

    const fin = component.financialSummary();
    expect(fin.totIva).toBe(0);
    // Lordo include bollo 2.00
    expect(fin.totaleLordo).toBe(502);
    expect(fin.importoRitenuta).toBe(0);
    expect(fin.nettoAPagare).toBe(502);
  });

  it("onSubmit blocca l'invio se il form e invalido", async () => {
    // Form is incomplete (missing required fields)
    expect(component.form.valid).toBe(false);

    await component.onSubmit();

    expect(invoiceStoreMock.createInvoice).not.toHaveBeenCalled();
    expect(notifyMock.error).toHaveBeenCalled();
  });

  it("includes service dates and preserves advanced line data when saving", () => {
    component.linee.at(0).patchValue({
      descrizione: "Consulting",
      quantita: 1,
      prezzoUnitario: 100,
      dataInizioPeriodo: "2026-03-01",
      dataFinePeriodo: "2026-03-31",
      fields: { scontoMaggiorazione: [{ tipo: "SC", percentuale: 10 }] },
    });

    const request = (component as any).buildRequest();
    expect(request.linee[0]).toMatchObject({
      dataInizioPeriodo: "2026-03-01",
      dataFinePeriodo: "2026-03-31",
      scontoMaggiorazione: [{ tipo: "SC", percentuale: 10 }],
    });
    expect(component.lineTotal(0)).toBe(90);
  });

  it("applies fixed discounts to the unit price before multiplying by quantity", () => {
    component.linee.at(0).patchValue({
      quantita: 2,
      prezzoUnitario: 100,
      fields: { scontoMaggiorazione: [{ tipo: "SC", importo: 10, percentuale: 5 }] },
    });

    expect(component.lineTotal(0)).toBe(180);
  });

  it("keeps the editor on the offending line and blocks saving and validation until corrected", async () => {
    component.form.patchValue({ numero: "1", cedenteId: "seller", cessionarioId: "buyer" });
    const description = component.linee.at(0).get("descrizione")!;
    description.setValue("Aggiornamento dell’immagine");
    component.currentStep.set(2);

    component.nextStep();
    expect(component.currentStep()).toBe(2);
    expect(description.touched).toBe(true);
    expect(description.hasError("sdiDescription")).toBe(true);

    component.currentStep.set(5);
    await component.onSubmit();
    expect(component.currentStep()).toBe(2);
    await component.validateDraft();
    expect(invoiceStoreMock.createInvoice).not.toHaveBeenCalled();
    expect(invoiceStoreMock.validateInvoice).not.toHaveBeenCalled();

    description.setValue("è".repeat(1001));
    expect(description.hasError("sdiDescription")).toBe(true);
    description.setValue("Qualità dell'immagine - rilascio");
    expect(component.form.valid).toBe(true);
    await component.onSubmit();
    expect(invoiceStoreMock.createInvoice).toHaveBeenCalledWith(expect.objectContaining({
      linee: [expect.objectContaining({ descrizione: "Qualità dell'immagine - rilascio" })],
    }));
  });

  it("configura correttamente un'autofattura TD17 con soggetto emittente CC e fattura collegata", () => {
    component.form.patchValue({
      tipoDocumento: "TD17",
      numero: "1/TD17",
      data: "2026-02-01",
      divisa: "EUR",
      cedenteId: "foreign-supplier",
      cessionarioId: "my-company",
      collegataNumero: "INV-US-0099",
      collegataData: "2026-01-20",
      modalitaPagamento: "MP08",
      importoPagamento: 122.0,
    });

    component.linee.at(0).patchValue({
      descrizione: "Server cloud hosting",
      quantita: 1,
      prezzoUnitario: 100,
      aliquotaIva: 22,
    });

    expect(component.isAutofattura()).toBe(true);

    const req = (component as any).buildRequest();
    expect(req.tipoDocumento).toBe("TD17");
    expect(req.isAutofattura).toBe(true);
    expect(req.soggettoEmittente).toBe("CC");
    expect(req.datiFattureCollegate).toEqual([
      { idDocumento: "INV-US-0099", data: "2026-01-20" },
    ]);
  });
});
