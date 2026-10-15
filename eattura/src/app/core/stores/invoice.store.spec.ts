import { describe, it, expect, vi, beforeEach } from "vitest";
import { Injector } from "@angular/core";
import { InvoiceStore } from "./invoice.store";
import { InvoiceService } from "../services/invoice.service";
import { ClientStore } from "./client.store";
import type { InvoiceSummary } from "../models/invoice.model";

describe("InvoiceStore", () => {
  const mockInvoices: InvoiceSummary[] = [
    {
      id: "inv-1",
      numero: "1",
      data: "2026-01-15",
      tipoDocumento: "TD01",
      importoTotale: 1000,
      cedenteDenominazione: "My Company",
      cessionarioDenominazione: "Client A",
      stato: "draft",
      direction: "sale",
    },
    {
      id: "inv-2",
      numero: "2",
      data: "2026-02-10",
      tipoDocumento: "TD01",
      importoTotale: 500,
      cedenteDenominazione: "Supplier B",
      cessionarioDenominazione: "My Company",
      stato: "validated",
      direction: "purchase",
    },
    {
      id: "inv-3",
      numero: "3",
      data: "2026-03-01",
      tipoDocumento: "TD01",
      importoTotale: 2000,
      cedenteDenominazione: "My Company",
      cessionarioDenominazione: "Client C",
      stato: "sent",
      direction: "sale",
    },
  ];

  let mockInvoiceService: {
    listInvoices: ReturnType<typeof vi.fn>;
    getInvoice: ReturnType<typeof vi.fn>;
    createInvoice: ReturnType<typeof vi.fn>;
    updateInvoice: ReturnType<typeof vi.fn>;
    deleteInvoice: ReturnType<typeof vi.fn>;
    setStatus: ReturnType<typeof vi.fn>;
    validateInvoice: ReturnType<typeof vi.fn>;
    exportXml: ReturnType<typeof vi.fn>;
    nextInvoiceNumber: ReturnType<typeof vi.fn>;
    sendInvoicePec: ReturnType<typeof vi.fn>;
    syncPec: ReturnType<typeof vi.fn>;
    pecSyncStatus: ReturnType<typeof vi.fn>;
    importXml: ReturnType<typeof vi.fn>;
    importFromDir: ReturnType<typeof vi.fn>;
    rectifyRejectedInvoice: ReturnType<typeof vi.fn>;
    importSdiReceipt: ReturnType<typeof vi.fn>;
  };

  let mockClientStore: {
    refresh: ReturnType<typeof vi.fn>;
  };

  let store: InvoiceStore;

  beforeEach(() => {
    mockInvoiceService = {
      listInvoices: vi.fn().mockResolvedValue(mockInvoices),
      getInvoice: vi.fn(),
      createInvoice: vi.fn(),
      updateInvoice: vi.fn(),
      deleteInvoice: vi.fn(),
      setStatus: vi.fn(),
      validateInvoice: vi.fn(),
      exportXml: vi.fn(),
      nextInvoiceNumber: vi.fn(),
      sendInvoicePec: vi.fn(),
      syncPec: vi.fn(),
      pecSyncStatus: vi.fn(),
      importXml: vi.fn(),
      importFromDir: vi.fn(),
      rectifyRejectedInvoice: vi.fn(),
      importSdiReceipt: vi.fn(),
    };

    mockClientStore = {
      refresh: vi.fn().mockResolvedValue([]),
    };

    const injector = Injector.create({
      providers: [
        InvoiceStore,
        { provide: InvoiceService, useValue: mockInvoiceService },
        { provide: ClientStore, useValue: mockClientStore },
      ],
    });

    store = injector.get(InvoiceStore);
  });

  it("should initialize with default counts and empty list", () => {
    expect(store.invoices()).toEqual([]);
    expect(store.totalCount()).toBe(0);
    expect(store.draftCount()).toBe(0);
    expect(store.validatedCount()).toBe(0);
    expect(store.salesCount()).toBe(0);
    expect(store.purchasesCount()).toBe(0);
  });

  it("should load invoices and compute reactive metrics", async () => {
    await store.loadAll();

    expect(store.invoices()).toHaveLength(3);
    expect(store.totalCount()).toBe(3);
    expect(store.draftCount()).toBe(1);
    expect(store.validatedCount()).toBe(1);
    expect(store.sentCount()).toBe(1);
    expect(store.salesCount()).toBe(2);
    expect(store.purchasesCount()).toBe(1);
  });

  it("should update status in-place reactively", async () => {
    await store.loadAll();
    mockInvoiceService.setStatus.mockResolvedValue({
      id: "inv-1",
      stato: "validated",
    } as any);

    await store.setStatus("inv-1", "validated");

    expect(store.draftCount()).toBe(0);
    expect(store.validatedCount()).toBe(2);
    const updated = store.invoices().find((i) => i.id === "inv-1");
    expect(updated?.stato).toBe("validated");
  });

  it("should delete invoice and remove it from store reactively", async () => {
    await store.loadAll();
    mockInvoiceService.deleteInvoice.mockResolvedValue(undefined);

    await store.deleteInvoice("inv-2");

    expect(store.invoices()).toHaveLength(2);
    expect(store.purchasesCount()).toBe(0);
    expect(store.totalCount()).toBe(2);
  });

  it("should rectify rejected invoice back to draft", async () => {
    await store.loadAll();
    mockInvoiceService.rectifyRejectedInvoice.mockResolvedValue({
      id: "inv-3",
      stato: "draft",
    } as any);

    await store.rectifyRejectedInvoice("inv-3");

    const rectified = store.invoices().find((i) => i.id === "inv-3");
    expect(rectified?.stato).toBe("draft");
  });

  it("should import SDI receipt and refresh list", async () => {
    mockInvoiceService.importSdiReceipt.mockResolvedValue({
      tipo: "RC",
      invoiceId: "inv-1",
      newStatus: "accepted",
    });

    const res = await store.importSdiReceipt("<RicevutaConsegna></RicevutaConsegna>");

    expect(mockInvoiceService.importSdiReceipt).toHaveBeenCalled();
    expect(res.tipo).toBe("RC");
  });

  it("refreshes invoices when a background PEC sync completes", async () => {
    await store.loadAll();
    mockInvoiceService.pecSyncStatus
      .mockResolvedValueOnce({ configured: true, lastAttemptAt: null, lastSuccessAt: null, nextSyncAt: 0, lastError: null })
      .mockResolvedValueOnce({ configured: true, lastAttemptAt: 2000, lastSuccessAt: 2000, nextSyncAt: 302000, lastError: null });

    await store.refreshPecSyncStatus();
    expect(mockInvoiceService.listInvoices).toHaveBeenCalledTimes(1);
    await store.refreshPecSyncStatus();
    expect(mockInvoiceService.listInvoices).toHaveBeenCalledTimes(2);
  });
});
