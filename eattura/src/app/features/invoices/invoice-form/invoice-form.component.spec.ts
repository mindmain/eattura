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
import type { ClientSummary } from "../../../core/models/client.model";

describe("InvoiceFormComponent", () => {
  let component: InvoiceFormComponent;
  let routerMock: { navigate: ReturnType<typeof vi.fn> };
  let notifyMock: { success: ReturnType<typeof vi.fn>; error: ReturnType<typeof vi.fn>; warning: ReturnType<typeof vi.fn> };
  let invoiceStoreMock: {
    getInvoice: ReturnType<typeof vi.fn>;
    createInvoice: ReturnType<typeof vi.fn>;
    updateInvoice: ReturnType<typeof vi.fn>;
    validateInvoice: ReturnType<typeof vi.fn>;
    nextInvoiceNumber: ReturnType<typeof vi.fn>;
  };
  let clientsSig: ReturnType<typeof signal<ClientSummary[]>>;
  let clientMapSig: ReturnType<typeof signal<Map<string, ClientSummary>>>;
  let clientStoreMock: {
    loadAll: ReturnType<typeof vi.fn>;
    clients: ReturnType<typeof signal<ClientSummary[]>>;
    clientMap: ReturnType<typeof signal<Map<string, ClientSummary>>>;
  };
  let settingsStoreMock: {
    load: ReturnType<typeof vi.fn>;
    findCompanyClientId: ReturnType<typeof vi.fn>;
    company: ReturnType<typeof signal<{ regime_fiscale: string } | null>>;
  };

  beforeEach(async () => {
    routerMock = { navigate: vi.fn() };
    notifyMock = { success: vi.fn(), error: vi.fn(), warning: vi.fn() };
    invoiceStoreMock = {
      getInvoice: vi.fn(),
      createInvoice: vi.fn().mockResolvedValue({ id: "new-inv-id" }),
      updateInvoice: vi.fn().mockResolvedValue(undefined),
      validateInvoice: vi.fn().mockResolvedValue({ valid: true, errors: [] }),
      nextInvoiceNumber: vi.fn().mockResolvedValue("1"),
    };
    clientsSig = signal<ClientSummary[]>([]);
    clientMapSig = signal(new Map<string, ClientSummary>());
    clientStoreMock = {
      loadAll: vi.fn().mockResolvedValue([]),
      clients: clientsSig,
      clientMap: clientMapSig,
    };
    settingsStoreMock = {
      load: vi.fn().mockResolvedValue(null),
      findCompanyClientId: vi.fn().mockReturnValue(null),
      company: signal(null),
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

  it("starts with 0 added lines and blocks advancing past Step 2 until a line is added", () => {
    expect(component.linee.length).toBe(0);
    expect(component.canProceedFromLines()).toBe(false);

    component.currentStep.set(2);
    component.nextStep();
    expect(component.currentStep()).toBe(2);
    expect(notifyMock.error).toHaveBeenCalled();

    // Add a line through the dedicated bottom composer
    component.lineDraftForm.patchValue({
      descrizione: "Consulenza strategica",
      quantita: 2,
      unitaMisura: "ore",
      prezzoUnitario: 80,
      aliquotaIva: 22,
    });
    expect(component.saveLineDraft()).toBe(true);
    expect(component.linee.length).toBe(1);
    expect(component.canProceedFromLines()).toBe(true);
    expect(component.runningTotal()).toBe(160);

    component.nextStep();
    expect(component.currentStep()).toBe(3);
  });

  it("supports editing and removing lines via the bottom form composer", () => {
    component.lineDraftForm.patchValue({
      descrizione: "Servizio base",
      quantita: 1,
      prezzoUnitario: 100,
      aliquotaIva: 22,
    });
    component.saveLineDraft();
    expect(component.linee.length).toBe(1);

    component.editLine(0);
    expect(component.editingLineIndex()).toBe(0);
    component.lineDraftForm.patchValue({
      descrizione: "Servizio avanzato",
      quantita: 3,
      prezzoUnitario: 50,
    });
    expect(component.saveLineDraft()).toBe(true);
    expect(component.editingLineIndex()).toBeNull();
    expect(component.linee.length).toBe(1);
    expect(component.linee.at(0).get("descrizione")?.value).toBe("Servizio avanzato");
    expect(component.runningTotal()).toBe(150);

    component.removeLine(0);
    expect(component.linee.length).toBe(0);
  });

  it("requires Natura when IVA is 0% and clears Natura when IVA > 0%", () => {
    component.lineDraftForm.patchValue({
      descrizione: "Prestazione esente",
      prezzoUnitario: 100,
      aliquotaIva: 0,
      natura: "",
    });
    expect(component.saveLineDraft()).toBe(false);
    expect(component.linee.length).toBe(0);

    component.lineDraftForm.patchValue({ natura: "N4" });
    expect(component.saveLineDraft()).toBe(true);
    expect(component.linee.at(0).get("natura")?.value).toBe("N4");

    // Selecting positive VAT automatically clears Natura
    component.lineDraftForm.patchValue({ natura: "N4" });
    component.lineDraftForm.get("aliquotaIva")?.setValue(22);
    expect(component.lineDraftForm.get("natura")?.value).toBe("");
  });

  it("calcola correttamente cassa previdenziale, ritenuta, bollo e netto a pagare", () => {
    component.addLine({
      descrizione: "Progettazione",
      quantita: 1,
      prezzoUnitario: 1000,
      aliquotaIva: 22,
    });

    component.form.patchValue({
      applicaCassa: true,
      tipoCassa: "TC04",
      alCassa: 4.0,
      cassaSoggettaIva: true,
      applicaRitenuta: true,
      tipoRitenuta: "RT01",
      aliquotaRitenuta: 20.0,
      causalePagamentoRitenuta: "A",
    });

    const fin = component.financialSummary();
    expect(fin.imponibileLinee).toBe(1000);
    expect(fin.contributoCassa).toBe(40);
    expect(fin.totIva).toBe(228.8);
    expect(fin.totaleLordo).toBe(1268.8);
    expect(fin.importoRitenuta).toBe(200);
    expect(fin.nettoAPagare).toBe(1068.8);

    component.syncPaymentAmount();
    expect(component.form.get("importoPagamento")?.value).toBe(1068.8);
  });

  it("auto-detects Cedente RF19 without polluting Step 1 Causale, auto-enables Bollo > 77.47, and warns if unchecked", () => {
    const rf19Seller: ClientSummary = {
      id: "seller-rf19",
      denominazione: "Mario Rossi Forfettario",
      nome: null,
      cognome: null,
      idPaese: "IT",
      idCodice: "12345678903",
      comune: "Milano",
      regimeFiscale: "RF19",
    };
    clientsSig.set([rf19Seller]);
    clientMapSig.set(new Map([[rf19Seller.id, rf19Seller]]));

    component.form.patchValue({ cedenteId: "seller-rf19" });
    expect(component.form.get("isForfettario")?.value).toBe(true);
    // Step 1 causale control stays untouched!
    expect(component.form.get("causale")?.value).toBe("");

    // Add a €500 line -> auto-activates Bollo Virtuale (> €77.47)
    component.lineDraftForm.patchValue({
      descrizione: "Consulenza sviluppo software",
      quantita: 1,
      prezzoUnitario: 500,
    });
    expect(component.saveLineDraft()).toBe(true);
    expect(component.linee.at(0).get("aliquotaIva")?.value).toBe(0);
    expect(component.linee.at(0).get("natura")?.value).toBe("N2.2");
    expect(component.form.get("applicaBollo")?.value).toBe(true);
    expect(component.financialSummary().bolloMissingWarning).toBe(false);
    expect(component.financialSummary().totaleLordo).toBe(502);

    // Unchecking Bollo Virtuale disables bolloAmt and triggers the mandatory warning
    component.form.patchValue({ applicaBollo: false });
    component.onBolloCheckboxChange();
    expect(component.financialSummary().bolloAmt).toBe(0);
    expect(component.financialSummary().bolloMissingWarning).toBe(true);

    // Re-checking restores €2.00 bollo and clears warning
    component.form.patchValue({ applicaBollo: true });
    component.onBolloCheckboxChange();
    expect(component.financialSummary().bolloAmt).toBe(2);
    expect(component.financialSummary().bolloMissingWarning).toBe(false);

    // Built request omits unknown `isAutofattura` field and splits statutory causale <= 200 chars
    component.form.patchValue({ numero: "1", cessionarioId: "seller-rf19" });
    const req = (component as unknown as { buildRequest: () => Record<string, unknown> }).buildRequest();
    expect("isAutofattura" in req).toBe(false);
    const causali = req["causale"] as string[];
    expect(causali.length).toBe(2);
    expect(causali.every((c) => c.length <= 200)).toBe(true);
    expect(causali.some((c) => c.includes("190/2014"))).toBe(true);
  });

  it("includes service dates, condizioniPagamento, and discounts when saving", () => {
    component.addLine({
      descrizione: "Consulting",
      quantita: 1,
      prezzoUnitario: 100,
      aliquotaIva: 22,
      dataInizioPeriodo: "2026-03-01",
      dataFinePeriodo: "2026-03-31",
      fields: { scontoMaggiorazione: [{ tipo: "SC", percentuale: 10 }] },
    });
    component.form.patchValue({
      numero: "1",
      cedenteId: "seller",
      cessionarioId: "buyer",
      condizioniPagamento: "TP01",
      modalitaPagamento: "MP05",
    });

    const request = (component as unknown as { buildRequest: () => any }).buildRequest();
    expect(request.linee[0]).toMatchObject({
      dataInizioPeriodo: "2026-03-01",
      dataFinePeriodo: "2026-03-31",
      scontoMaggiorazione: [{ tipo: "SC", percentuale: 10 }],
    });
    expect(request.pagamenti[0].condizioniPagamento).toBe("TP01");
    expect(component.lineTotal(0)).toBe(90);
  });

  it("applies fixed discounts to the unit price before multiplying by quantity", () => {
    component.addLine({
      descrizione: "Item",
      quantita: 2,
      prezzoUnitario: 100,
      aliquotaIva: 22,
      fields: { scontoMaggiorazione: [{ tipo: "SC", importo: 10, percentuale: 5 }] },
    });

    expect(component.lineTotal(0)).toBe(180);
  });

  it("keeps the editor on the offending line and blocks saving and validation until corrected", async () => {
    component.form.patchValue({ numero: "1", cedenteId: "seller", cessionarioId: "buyer" });
    component.addLine({
      descrizione: "Aggiornamento dell’immagine",
      quantita: 1,
      prezzoUnitario: 100,
      aliquotaIva: 22,
    });
    const description = component.linee.at(0).get("descrizione")!;
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
    expect(invoiceStoreMock.createInvoice).toHaveBeenCalledWith(
      expect.objectContaining({
        linee: [expect.objectContaining({ descrizione: "Qualità dell'immagine - rilascio" })],
      }),
    );
  });

  it("configura correttamente un'autofattura TD17 con soggetto emittente CC senza inviare isAutofattura", () => {
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

    component.addLine({
      descrizione: "Server cloud hosting",
      quantita: 1,
      prezzoUnitario: 100,
      aliquotaIva: 22,
    });

    expect(component.isAutofattura()).toBe(true);

    const req = (component as unknown as { buildRequest: () => any }).buildRequest();
    expect(req.tipoDocumento).toBe("TD17");
    expect("isAutofattura" in req).toBe(false);
    expect(req.soggettoEmittente).toBe("CC");
    expect(req.datiFattureCollegate).toEqual([
      { idDocumento: "INV-US-0099", data: "2026-01-20" },
    ]);
  });
});
