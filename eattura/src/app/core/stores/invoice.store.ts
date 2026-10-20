import { Injectable, computed, inject, signal } from "@angular/core";
import { InvoiceService } from "../services/invoice.service";
import type { PecSyncStatus } from "../services/invoice.service";
import { ClientStore } from "./client.store";
import { AppLogService } from "../services/app-log.service";
import type {
  InvoiceSummary,
  InvoiceDetail,
  InvoiceStatus,
  CreateInvoiceRequest,
  DirImportSummary,
  NotificationSummary,
} from "../models/invoice.model";

/**
 * Reactive Signal Store for Invoices.
 *
 * Centralizes invoice state across Dashboard and Invoice List, avoiding
 * redundant IPC calls and ensuring that imports or status transitions
 * immediately reflect in all active views.
 */
@Injectable({ providedIn: "root" })
export class InvoiceStore {
  private invoiceService = inject(InvoiceService);
  private clientStore = inject(ClientStore);
  private readonly log = inject(AppLogService);

  private _invoices = signal<InvoiceSummary[]>([]);
  private _loading = signal<boolean>(false);
  private hasLoaded = false;

  // Readonly signals exposed to components
  readonly invoices = this._invoices.asReadonly();
  readonly loading = this._loading.asReadonly();
  private _pecSyncStatus = signal<PecSyncStatus | null>(null);
  private _clock = signal(Date.now());
  readonly pecSyncStatus = this._pecSyncStatus.asReadonly();
  readonly pecSyncError = signal<string | null>(null);
  readonly lastPecSuccessAt = computed(() => this._pecSyncStatus()?.lastSuccessAt ?? null);
  readonly secondsToPecSync = computed(() => {
    const status = this._pecSyncStatus();
    return status?.configured && !status.inProgress ? Math.max(0, Math.ceil((status.nextSyncAt - this._clock()) / 1000)) : null;
  });
  private pecTimer?: ReturnType<typeof setInterval>;
  private pecPoll?: ReturnType<typeof setInterval>;
  private pecStatusPending = false;

  startPecMonitor(): void {
    if (this.pecTimer) return;
    void this.log.record("info", "pec-monitor", "PEC countdown and status monitor started");
    this._clock.set(Date.now());
    void this.refreshPecSyncStatus();
    this.pecTimer = setInterval(() => this._clock.set(Date.now()), 1000);
    this.pecPoll = setInterval(() => void this.refreshPecSyncStatus(), 15_000);
  }

  stopPecMonitor(): void {
    if (this.pecTimer) clearInterval(this.pecTimer);
    if (this.pecPoll) clearInterval(this.pecPoll);
    this.pecTimer = undefined;
    this.pecPoll = undefined;
  }

  async refreshPecSyncStatus(): Promise<void> {
    if (this.pecStatusPending) return;
    this.pecStatusPending = true;
    let timeout: ReturnType<typeof setTimeout> | undefined;
    try {
      const status = await Promise.race([
        this.invoiceService.pecSyncStatus(),
        new Promise<never>((_, reject) => {
          timeout = setTimeout(() => reject(new Error("PEC status request timed out after 10 seconds")), 10_000);
        }),
      ]);
      if (!status || typeof status.configured !== "boolean" || !Number.isFinite(status.nextSyncAt)) {
        throw new Error("Invalid PEC status returned by desktop backend");
      }
      this.pecSyncError.set(null);
      const previous = this._pecSyncStatus();
      this._pecSyncStatus.set(status);
      if (this.hasLoaded && status.lastSuccessAt && status.lastSuccessAt !== previous?.lastSuccessAt) {
        await this.refresh();
      }
    } catch (error) {
      if (!this.pecSyncError()) void this.log.record("error", "pec-monitor", error);
      this.pecSyncError.set("Impossibile leggere lo stato della PEC. Controlla il registro attività.");
    } finally {
      if (timeout !== undefined) clearTimeout(timeout);
      this.pecStatusPending = false;
    }
  }

  // Global derived metrics
  readonly totalCount = computed(() => this._invoices().length);
  readonly draftCount = computed(
    () => this._invoices().filter((i) => i.stato === "draft").length
  );
  readonly validatedCount = computed(
    () => this._invoices().filter((i) => i.stato === "validated").length
  );
  readonly sentCount = computed(
    () => this._invoices().filter((i) => i.stato === "sent").length
  );
  readonly acceptedCount = computed(
    () => this._invoices().filter((i) => i.stato === "accepted").length
  );
  readonly rejectedCount = computed(
    () => this._invoices().filter((i) => i.stato === "rejected").length
  );

  readonly salesCount = computed(
    () => this._invoices().filter((i) => i.direction === "sale").length
  );
  readonly purchasesCount = computed(
    () => this._invoices().filter((i) => i.direction === "purchase").length
  );

  /**
   * Load all invoices into store.
   * If already loaded and force is false, reuses in-memory cache.
   */
  async loadAll(force = false): Promise<InvoiceSummary[]> {
    if (this.hasLoaded && !force) {
      return this._invoices();
    }
    this._loading.set(true);
    try {
      const list = await this.invoiceService.listInvoices();
      this._invoices.set(list);
      this.hasLoaded = true;
      return list;
    } finally {
      this._loading.set(false);
    }
  }

  /** Force reload invoices from backend. */
  async refresh(): Promise<InvoiceSummary[]> {
    return this.loadAll(true);
  }

  /** Fetch a single invoice detail by ID. */
  async getInvoice(id: string): Promise<InvoiceDetail> {
    return this.invoiceService.getInvoice(id);
  }

  /** Create a new invoice and refresh the in-memory list. */
  async createInvoice(invoice: CreateInvoiceRequest): Promise<InvoiceDetail> {
    const detail = await this.invoiceService.createInvoice(invoice);
    await this.refresh();
    return detail;
  }

  /** Update a draft invoice and refresh the in-memory list. */
  async updateInvoice(
    id: string,
    invoice: CreateInvoiceRequest
  ): Promise<InvoiceDetail> {
    const detail = await this.invoiceService.updateInvoice(id, invoice);
    await this.refresh();
    return detail;
  }

  /** Delete a draft invoice and remove it from store. */
  async deleteInvoice(id: string): Promise<void> {
    await this.invoiceService.deleteInvoice(id);
    this._invoices.update((list) => list.filter((i) => i.id !== id));
  }

  /** Change lifecycle status and update in-memory item. */
  async setStatus(id: string, status: InvoiceStatus): Promise<InvoiceDetail> {
    const detail = await this.invoiceService.setStatus(id, status);
    this._invoices.update((list) =>
      list.map((inv) => (inv.id === id ? { ...inv, stato: status } : inv))
    );
    return detail;
  }

  /** Validate invoice against SDI rules. */
  async validateInvoice(
    id: string
  ): Promise<{ valid: boolean; errors: { code: string; message: string }[] }> {
    return this.invoiceService.validateInvoice(id);
  }

  /** Export invoice to XML string. */
  async exportXml(id: string): Promise<string> {
    return this.invoiceService.exportXml(id);
  }

  /** Render invoice PDF courtesy copy. */
  async renderPdf(id: string): Promise<Uint8Array> {
    const bytes = await this.invoiceService.renderPdf(id);
    return new Uint8Array(bytes);
  }

  /** Suggest the next progressive invoice number. */
  async nextInvoiceNumber(cedenteId: string, year: string): Promise<string> {
    return this.invoiceService.nextInvoiceNumber(cedenteId, year);
  }

  /** Send a validated invoice to SDI via PEC and mark it as sent. */
  async sendInvoicePec(id: string): Promise<string> {
    const response = await this.invoiceService.sendInvoicePec(id);
    this._invoices.update((list) =>
      list.map((inv) => (inv.id === id ? { ...inv, stato: "sent" } : inv))
    );
    return response;
  }

  /** Sync PEC invoices and receipts, then refresh the visible archive. */
  async syncPec(): Promise<NotificationSummary[]> {
    try {
      const summaries = await this.invoiceService.syncPec();
      if (summaries.length > 0) {
        await this.refresh();
      }
      return summaries;
    } finally {
      await this.refreshPecSyncStatus();
    }
  }

  /**
   * Import a single XML invoice file.
   * Refreshes both invoices and clients (since new clients are auto-created).
   */
  async importXml(xmlContent: string): Promise<InvoiceDetail> {
    const detail = await this.invoiceService.importXml(xmlContent);
    await Promise.all([this.refresh(), this.clientStore.refresh()]);
    return detail;
  }

  /**
   * Import a directory of XML invoice files.
   * Refreshes both invoices and clients (since new clients are auto-created).
   */
  async importFromDir(dirPath: string): Promise<DirImportSummary> {
    const summary = await this.invoiceService.importFromDir(dirPath);
    if (summary.imported > 0) {
      await Promise.all([this.refresh(), this.clientStore.refresh()]);
    }
    return summary;
  }

  /** Sign invoice into CAdES-BES .p7m envelope. */
  async signP7m(id: string, signerName?: string, signerCf?: string): Promise<Uint8Array> {
    const bytes = await this.invoiceService.signP7m(id, signerName, signerCf);
    await this.refresh();
    return new Uint8Array(bytes);
  }

  /** Rectify a rejected invoice (SDI scarto), moving it back to draft. */
  async rectifyRejectedInvoice(id: string): Promise<InvoiceDetail> {
    const detail = await this.invoiceService.rectifyRejectedInvoice(id);
    this._invoices.update((list) =>
      list.map((inv) => (inv.id === id ? { ...inv, stato: "draft" } : inv))
    );
    return detail;
  }

  /** Import an SDI receipt XML and link it to its invoice dossier. */
  async importSdiReceipt(xmlContent: string): Promise<NotificationSummary> {
    const summary = await this.invoiceService.importSdiReceipt(xmlContent);
    await this.refresh();
    return summary;
  }

  /** Generate a draft autofattura / reverse charge integration for a purchase invoice. */
  async generateAutofatturaDraft(
    purchaseInvoiceId: string,
    autofatturaType?: string
  ): Promise<CreateInvoiceRequest> {
    return this.invoiceService.generateAutofatturaDraft(
      purchaseInvoiceId,
      autofatturaType
    );
  }
}
