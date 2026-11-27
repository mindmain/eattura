import { readFileSync } from "node:fs";
import { signal } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { provideRouter } from "@angular/router";
import { describe, expect, it, vi } from "vitest";
import type { InvoiceSummary } from "../../../core/models/invoice.model";
import type { PecSyncProgress, PecSyncStatus } from "../../../core/services/invoice.service";
import { FileDialogService } from "../../../core/services/file-dialog.service";
import { NotificationService } from "../../../core/services/notification.service";
import { InvoiceStore } from "../../../core/stores/invoice.store";
import { InvoiceListComponent } from "./invoice-list.component";

function makeInvoice(
  id: string,
  stato: InvoiceSummary["stato"],
  numero = "1"
): InvoiceSummary {
  return {
    id,
    numero,
    data: "2026-03-10",
    tipoDocumento: "TD01",
    importoTotale: 100,
    cedenteDenominazione: "Seller SRL",
    cessionarioDenominazione: "Buyer SPA",
    direction: "sale",
    stato,
  };
}

describe("InvoiceListComponent", () => {
  async function setup(
    records: InvoiceSummary[],
    pecStatus: PecSyncStatus | null = null,
    pecProgress: PecSyncProgress | null = null,
  ) {
    const invoices = signal(records);
    const pecSyncStatus = signal<PecSyncStatus | null>(pecStatus);
    const pecSyncProgress = signal<PecSyncProgress | null>(pecProgress);
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      providers: [
        provideRouter([]),
        {
          provide: InvoiceStore,
          useValue: {
            invoices,
            loading: signal(false),
            pecSyncStatus,
            pecSyncProgress,
            pecSyncError: signal(null),
            secondsToPecSync: signal(null),
            loadAll: vi.fn().mockResolvedValue(records),
          },
        },
        {
          provide: FileDialogService,
          useValue: {
            pickAndReadXmlFile: vi.fn(),
            pickDirectory: vi.fn(),
            confirm: vi.fn(),
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
    TestBed.overrideComponent(InvoiceListComponent, {
      set: {
        templateUrl: undefined,
        styleUrl: undefined,
        template: readFileSync(
          "src/app/features/invoices/invoice-list/invoice-list.component.html",
          "utf8"
        ),
        styles: [],
      },
    });
    await TestBed.compileComponents();
    const fixture = TestBed.createComponent(InvoiceListComponent);
    fixture.detectChanges();
    return { fixture, component: fixture.componentInstance };
  }

  it("does not expose a dedicated Consegnate tab and includes delivered invoices in Accettate", async () => {
    const { fixture, component } = await setup([
      makeInvoice("inv-draft", "draft", "1"),
      makeInvoice("inv-accepted", "accepted", "2"),
      makeInvoice("inv-delivered", "delivered", "3"),
      makeInvoice("inv-sent", "sent", "4"),
    ]);

    expect(component.statusTabs.map((t) => t.value)).not.toContain("delivered");
    expect(component.statusTabs.map((t) => t.label)).not.toContain("Consegnate");

    component.setStatusFilter("accepted");
    fixture.detectChanges();

    expect(component.filteredInvoices().map((i) => i.id)).toEqual([
      "inv-accepted",
      "inv-delivered",
    ]);

    const deliveredBadge = fixture.nativeElement.querySelector(".badge-delivered");
    expect(deliveredBadge).not.toBeNull();
    expect(deliveredBadge.textContent).toContain("Consegnata");
    const checkIcon = deliveredBadge.querySelector("svg.delivered-check");
    expect(checkIcon).not.toBeNull();
    expect(checkIcon.getAttribute("aria-hidden")).toBe("true");
  });

  it("keeps sent invoices with MC notice under Inviate while showing the MC warning badge", async () => {
    const mcInvoice: InvoiceSummary = {
      ...makeInvoice("inv-b2b-mc", "sent", "5"),
      isPa: false,
      hasMcNotice: true,
    };
    const pendingSent = makeInvoice("inv-pending", "sent", "6");
    const { fixture, component } = await setup([mcInvoice, pendingSent]);

    component.setStatusFilter("sent");
    fixture.detectChanges();
    expect(component.filteredInvoices().map((i) => i.id)).toEqual([
      "inv-b2b-mc",
      "inv-pending",
    ]);

    const mcBadge = fixture.nativeElement.querySelector(".badge-mc");
    expect(mcBadge).not.toBeNull();
    expect(mcBadge.textContent).toContain("Mancata consegna (MC)");

    component.setStatusFilter("accepted");
    fixture.detectChanges();
    expect(component.filteredInvoices()).toEqual([]);
  });

  it("shows live PEC read and processed counters and progress bar during sync", async () => {
    const { fixture } = await setup(
      [makeInvoice("inv-1", "sent", "6")],
      {
        configured: true,
        inProgress: true,
        lastAttemptAt: 1000,
        lastSuccessAt: null,
        nextSyncAt: 301000,
        lastError: null,
        sdiAddressDetected: "sdi48@pec.fatturapa.it",
        sdiAddressVerified: true,
      },
      {
        phase: "processing",
        readMessages: 207,
        totalMessages: 207,
        processedMessages: 84,
        processedItems: 3,
      },
    );

    const statusBar = fixture.nativeElement.querySelector(".pec-sync-timer");
    expect(statusBar).not.toBeNull();
    expect(statusBar.textContent).toContain("207 / 207 letti");
    expect(statusBar.textContent).toContain("84 / 207 svolti");
    expect(statusBar.textContent).toContain("3 documenti registrati");

    const progressBar = fixture.nativeElement.querySelector(".pec-progress-track");
    expect(progressBar).not.toBeNull();
  });

  it("surfaces Ricevuta da SdI substatus when PEC reached SdI and RC is still pending", async () => {
    const { fixture } = await setup([
      {
        ...makeInvoice("inv-sdi", "sent", "6"),
        pecDeliveredToSdiAt: "27/11/2026 11:21:24",
      },
    ]);

    const substatus = fixture.nativeElement.querySelector(".pec-sdi-substatus");
    expect(substatus).not.toBeNull();
    expect(substatus.textContent).toContain("Ricevuta da SdI · attesa RC");
  });
});
