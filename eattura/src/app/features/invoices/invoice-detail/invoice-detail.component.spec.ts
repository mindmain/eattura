import { readFileSync } from "node:fs";
import { signal } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { provideRouter } from "@angular/router";
import { describe, expect, it, vi } from "vitest";
import type { InvoiceDetail } from "../../../core/models/invoice.model";
import { FileDownloadService } from "../../../core/services/file-download.service";
import { NotificationService } from "../../../core/services/notification.service";
import { InvoiceStore } from "../../../core/stores/invoice.store";
import { InvoiceDetailComponent } from "./invoice-detail.component";

function buildDetail(overrides: Partial<InvoiceDetail>): InvoiceDetail {
  return {
    id: "inv-1",
    numero: "42",
    data: "2026-03-10",
    tipoDocumento: "TD01",
    divisa: "EUR",
    importoTotale: 122,
    cedenteId: "seller",
    cessionarioId: "buyer",
    cedenteDenominazione: "Seller SRL",
    cessionarioDenominazione: "Comune di Prova",
    direction: "sale",
    stato: "rejected",
    causale: [],
    linee: [],
    pagamenti: [],
    notifications: [],
    ...overrides,
  };
}

describe("InvoiceDetailComponent rejection handling", () => {
  async function setup(detail: InvoiceDetail) {
    const rectifyRejectedInvoice = vi.fn();
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      providers: [
        provideRouter([]),
        {
          provide: InvoiceStore,
          useValue: {
            getInvoice: vi.fn().mockResolvedValue(detail),
            lastPecSuccessAt: signal(null),
            mcpDraft: signal(null),
            pecSyncStatus: signal(null),
            rectifyRejectedInvoice,
          },
        },
        {
          provide: FileDownloadService,
          useValue: {
            downloadTextFile: vi.fn(),
            downloadBinaryFile: vi.fn(),
          },
        },
        {
          provide: NotificationService,
          useValue: {
            success: vi.fn(),
            error: vi.fn(),
          },
        },
      ],
    });
    TestBed.overrideComponent(InvoiceDetailComponent, {
      set: {
        templateUrl: undefined,
        styleUrl: undefined,
        template: readFileSync(
          "src/app/features/invoices/invoice-detail/invoice-detail.component.html",
          "utf8",
        ),
        styles: [],
      },
    });
    await TestBed.compileComponents();
    const fixture = TestBed.createComponent(InvoiceDetailComponent);
    fixture.componentInstance.invoice.set(detail);
    fixture.detectChanges();
    return { fixture, component: fixture.componentInstance, rectifyRejectedInvoice };
  }

  it("distinguishes PA refusal (RC -> NE EC02) from SDI discard (NS) and hides draft rectification", async () => {
    const paRefused = buildDetail({
      isPa: true,
      formatoTrasmissione: "FPA12",
      activePecFilename: "IT01234567897_00001.xml",
      notifications: [
        {
          id: "rc-1",
          tipo: "RC",
          identificativoSdi: "888222",
          nomeFile: "IT01234567897_00001.xml",
          dataOraRicezione: "2026-03-10T10:00:00Z",
          errori: [],
          rawXml: "<RicevutaConsegna/>",
        },
        {
          id: "ne-1",
          tipo: "NE",
          identificativoSdi: "888222",
          nomeFile: "IT01234567897_00001.xml",
          esito: "EC02",
          note: "Codice CIG mancante",
          dataOraRicezione: "2026-03-12T11:30:00Z",
          errori: [],
          rawXml: "<NotificaEsito/>",
        },
      ],
    });

    const { fixture, component } = await setup(paRefused);

    expect(component.paRefusalInfo()?.notification.esito).toBe("EC02");
    expect(component.scartoInfo()).toBeNull();
    expect(component.canRectifyRejected()).toBe(false);

    const text = fixture.nativeElement.textContent as string;
    expect(text).toContain("NOTIFICA DI ESITO NEGATIVO PA (NE - EC02)");
    expect(text).toContain("Codice CIG mancante");
    expect(text).toContain("Rifiutata dalla PA (EC02)");
    expect(text).not.toContain("NOTIFICA DI SCARTO SDI (NS)");
    expect(fixture.nativeElement.querySelector(".btn-rectify")).toBeNull();
  });

  it("shows the 5-day scarto banner and rectification button for SDI NS discards", async () => {
    const recentIso = new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString();
    const nsDiscarded = buildDetail({
      isPa: false,
      formatoTrasmissione: "FPR12",
      activePecFilename: "IT01234567897_00002.xml",
      notifications: [
        {
          id: "ns-1",
          tipo: "NS",
          identificativoSdi: "999111",
          nomeFile: "IT01234567897_00002.xml",
          dataOraRicezione: recentIso,
          errori: [{ codice: "00400", descrizione: "Natura non presente" }],
          rawXml: "<NotificaScarto/>",
        },
      ],
    });

    const { fixture, component } = await setup(nsDiscarded);

    expect(component.paRefusalInfo()).toBeNull();
    expect(component.canRectifyRejected()).toBe(true);
    expect(component.scartoInfo()?.daysRemaining).toBe(4);

    const text = fixture.nativeElement.textContent as string;
    expect(text).toContain("NOTIFICA DI SCARTO SDI (NS)");
    expect(text).toContain("4 giorni rimanenti");
    expect(fixture.nativeElement.querySelectorAll(".btn-rectify").length).toBe(2);
  });
});
