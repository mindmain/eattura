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
    const syncPec = vi.fn().mockResolvedValue([]);
    const getInvoice = vi.fn().mockResolvedValue(detail);
    const exportXml = vi.fn().mockResolvedValue(
      "<FatturaElettronicaHeader><DatiTrasmissione><IdTrasmittente><IdPaese>IT</IdPaese><IdCodice>01234567897</IdCodice></IdTrasmittente><ProgressivoInvio>A1B2C</ProgressivoInvio></DatiTrasmissione></FatturaElettronicaHeader>",
    );
    const renderPdf = vi.fn().mockResolvedValue(new Uint8Array([37, 80, 68, 70]));
    const downloadTextFile = vi.fn();
    const downloadBinaryFile = vi.fn();
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      providers: [
        provideRouter([]),
        {
          provide: InvoiceStore,
          useValue: {
            getInvoice,
            syncPec,
            lastPecSuccessAt: signal(null),
            mcpDraft: signal(null),
            pecSyncActive: signal(false),
            pecSyncLabel: signal(""),
            pecSyncStatus: signal({
              configured: true,
              lastAttemptAt: null,
              lastSuccessAt: null,
              nextSyncAt: 0,
              lastError: null,
              sdiAddress: "sdi48@pec.fatturapa.it",
              sdiAddressVerified: true,
            }),
            rectifyRejectedInvoice,
            exportXml,
            renderPdf,
          },
        },
        {
          provide: FileDownloadService,
          useValue: {
            downloadTextFile,
            downloadBinaryFile,
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
    return {
      fixture,
      component: fixture.componentInstance,
      rectifyRejectedInvoice,
      syncPec,
      getInvoice,
      exportXml,
      renderPdf,
      downloadTextFile,
      downloadBinaryFile,
    };
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

  it("downloads XML and PDF using the transmitted SdI filename stem (including .xml.p7m imports)", async () => {
    const signedImport = buildDetail({
      stato: "delivered",
      activePecFilename: "IT01234567897_9X2K1.xml.p7m",
      sdiFilename: "IT01234567897_9X2K1.xml",
    });

    const { component, downloadTextFile, downloadBinaryFile } = await setup(signedImport);
    await component.exportXml();
    await component.downloadPdf();

    expect(downloadTextFile).toHaveBeenCalledWith(
      expect.any(String),
      "IT01234567897_9X2K1.xml",
      "application/xml",
    );
    expect(downloadBinaryFile).toHaveBeenCalledWith(
      expect.any(Uint8Array),
      "IT01234567897_9X2K1.pdf",
      "application/pdf",
    );
  });

  it("downloads draft XML and PDF using the generated SdI filename (<IdPaese><IdCodice>_<Progressivo>)", async () => {
    const draft = buildDetail({
      stato: "draft",
      numero: "2026/42",
      activePecFilename: null,
      sdiFilename: "IT01234567897_A1B2C.xml",
    });

    const { component, downloadTextFile, downloadBinaryFile } = await setup(draft);
    await component.exportXml();
    await component.downloadPdf();

    expect(downloadTextFile).toHaveBeenCalledWith(
      expect.any(String),
      "IT01234567897_A1B2C.xml",
      "application/xml",
    );
    expect(downloadBinaryFile).toHaveBeenCalledWith(
      expect.any(Uint8Array),
      "IT01234567897_A1B2C.pdf",
      "application/pdf",
    );
  });

  it("distinguishes PEC delivery to SdI from SdI RC delivery to customer and refreshes via syncPecNow", async () => {
    const sent = buildDetail({
      stato: "sent",
      activePecFilename: "ITSPNNCI93R07C351G_74660.xml",
      sdiFilename: "ITSPNNCI93R07C351G_74660.xml",
      pecDeliveredToSdiAt: "27/11/2026 11:21:25",
      pecSdiRecipient: "sdi48@pec.fatturapa.it",
    });

    const { fixture, component, syncPec, getInvoice } = await setup(sent);
    const text = fixture.nativeElement.textContent as string;
    expect(text).toContain("Busta PEC consegnata al Sistema di Interscambio");
    expect(text).toContain("sdi48@pec.fatturapa.it");
    expect(text).toContain("27/11/2026 11:21:25");
    expect(text).toContain("Ricevuta di Consegna (RC)");

    const delivered = buildDetail({
      ...sent,
      stato: "delivered",
      notifications: [
        {
          id: "rc-74660",
          tipo: "RC",
          identificativoSdi: "18999999",
          nomeFile: "ITSPNNCI93R07C351G_74660.XML",
          dataOraRicezione: "2026-11-27T13:05:00Z",
          errori: [],
          rawXml: "<RicevutaConsegna/>",
        },
      ],
    });
    syncPec.mockResolvedValueOnce([
      {
        tipo: "RC",
        identificativoSdi: "18999999",
        nomeFile: "ITSPNNCI93R07C351G_74660.XML",
        invoiceId: sent.id,
        newStatus: "delivered",
      },
    ]);
    getInvoice.mockResolvedValueOnce(delivered);

    await component.syncPecNow();
    fixture.detectChanges();

    expect(component.invoice()?.stato).toBe("delivered");
    expect(fixture.nativeElement.textContent).toContain("Consegnata al destinatario");
  });
});
