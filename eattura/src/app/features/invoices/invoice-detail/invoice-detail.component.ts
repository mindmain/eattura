import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  input,
  signal,
} from "@angular/core";
import { Router, RouterLink } from "@angular/router";
import { ask } from "@tauri-apps/plugin-dialog";
import { InvoiceStore } from "../../../core/stores/invoice.store";
import { NotificationService } from "../../../core/services/notification.service";
import { FileDownloadService } from "../../../core/services/file-download.service";
import { TipoDocumentoPipe } from "../../../shared/pipes/tipo-documento.pipe";
import { InvoiceStatusPipe } from "../../../shared/pipes/invoice-status.pipe";
import { AppCurrencyPipe } from "../../../shared/pipes/app-currency.pipe";
import { ModalitaPagamentoPipe } from "../../../shared/pipes/modalita-pagamento.pipe";
import type {
  InvoiceDetail,
  InvoiceStatus,
  SdiNotificationDetail,
  AttachmentDetail,
} from "../../../core/models/invoice.model";

@Component({
  selector: "app-invoice-detail",
  standalone: true,
  imports: [
    RouterLink,
    TipoDocumentoPipe,
    InvoiceStatusPipe,
    AppCurrencyPipe,
    ModalitaPagamentoPipe,
  ],
  templateUrl: "./invoice-detail.component.html",
  styleUrl: "./invoice-detail.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class InvoiceDetailComponent {
  private invoiceStore = inject(InvoiceStore);
  private fileDownload = inject(FileDownloadService);
  private router = inject(Router);
  private notify = inject(NotificationService);

  /** Router input binding via withComponentInputBinding */
  id = input.required<string>();

  invoice = signal<InvoiceDetail | null>(null);
  validationResult = signal<{ valid: boolean; errors: { code: string; message: string }[] } | null>(null);
  sending = signal(false);
  downloadingPdf = signal(false);
  showAdeView = signal(false);
  rectifying = signal(false);
  selectedReceipt = signal<SdiNotificationDetail | null>(null);
  generatingAutofattura = signal(false);

  isAutofattura = computed(() => {
    const inv = this.invoice();
    if (!inv) return false;
    return !!(
      inv.isAutofattura ||
      ["TD16", "TD17", "TD18", "TD19", "TD20", "TD21", "TD27"].includes(inv.tipoDocumento)
    );
  });

  isPurchase = computed(() => {
    const inv = this.invoice();
    return inv?.direction === "purchase";
  });

  /** Scarto calculation for the 5-day rectification window (Circolare AdE 13/E 2018). */
  scartoInfo = computed(() => {
    const inv = this.invoice();
    if (!inv || inv.stato !== "rejected") return null;

    if (inv.recoverablePecFilename) return null;
    const ns = this.currentAttemptNotifications().find((n) => n.tipo === "NS");
    if (!ns) return { notification: null, daysRemaining: 5, expired: false };

    let daysRemaining = 5;
    let expired = false;

    if (ns.dataOraRicezione) {
      const receiptDate = new Date(ns.dataOraRicezione);
      if (!isNaN(receiptDate.getTime())) {
        const diffMs = Date.now() - receiptDate.getTime();
        const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));
        daysRemaining = Math.max(0, 5 - diffDays);
        expired = diffDays > 5;
      }
    }

    return { notification: ns, daysRemaining, expired };
  });

  currentAttemptNotifications = computed(() => {
    const inv = this.invoice();
    if (!inv || inv.stato === "draft" || inv.stato === "validated" || inv.recoverablePecFilename) {
      return [];
    }
    const notifications = inv.notifications ?? [];
    const active = inv.activePecFilename;
    if (!active) return notifications;
    const stem = (filename: string) => filename.replace(/(?:\.xml)?\.p7m$|\.xml$/i, "").toLowerCase();
    const matchesFile = (filename?: string) => !!filename && stem(filename) === stem(active);
    const ids = new Set(notifications.filter((receipt) => matchesFile(receipt.nomeFile))
      .map((receipt) => receipt.identificativoSdi).filter(Boolean));
    return notifications.filter((receipt) => matchesFile(receipt.nomeFile)
      || (!receipt.nomeFile && ids.has(receipt.identificativoSdi)));
  });

  latestNotification = computed(() => this.currentAttemptNotifications().at(-1) ?? null);

  hasDdt = computed(() => (this.invoice()?.datiDdt?.length ?? 0) > 0);
  hasAllegati = computed(() => (this.invoice()?.allegati?.length ?? 0) > 0);
  readonly pecSyncStatus = this.invoiceStore.pecSyncStatus;

  constructor() {
    effect((onCleanup) => {
      const invoiceId = this.id();
      // Refresh the open fascicolo when the background PEC sync completes.
      this.invoiceStore.lastPecSuccessAt();
      if (!invoiceId) return;
      const draft = this.invoiceStore.mcpDraft();
      if (draft?.previousId === invoiceId && draft.id !== invoiceId) {
        void this.router.navigate(["/invoices", draft.id], { replaceUrl: true });
        return;
      }

      let active = true;
      onCleanup(() => { active = false; });
      void this.invoiceStore.getInvoice(invoiceId).then(
        (invoice) => { if (active) this.invoice.set(invoice); },
        () => {
          if (!active) return;
          this.notify.error("Errore nel caricamento della fattura");
          void this.router.navigate(["/invoices"]);
        },
      );
    });
  }

  taxIdErrorLabel(path: string | null): string {
    const field = path?.endsWith("/CodiceFiscale") ? "Codice fiscale" : "Partita IVA";
    if (path?.includes("/RappresentanteFiscale/")) return `${field} del rappresentante fiscale`;
    if (path?.includes("/TerzoIntermediarioOSoggettoEmittente/")) return `${field} dell’intermediario`;
    if (path?.includes("/CedentePrestatore/")) return `${field} del cedente`;
    return `${field} del cessionario`;
  }

  async validate(): Promise<void> {
    const inv = this.invoice();
    if (!inv) return;
    try {
      const result = await this.invoiceStore.validateInvoice(inv.id);
      this.validationResult.set(result);
      if (result.valid) {
        this.notify.success("Fattura valida");
      } else {
        this.notify.error(`Validazione fallita: ${result.errors.length} errori`);
      }
    } catch {
      this.notify.error("Errore durante la validazione");
    }
  }

  /** Promote a draft or imported invoice to `validated` (the backend blocks it if SDI validation fails). */
  async promoteToValidated(): Promise<void> {
    await this.changeStatus("validated", "Fattura validata");
  }

  /** Revert a validated invoice back to `draft` to edit it again. */
  async revertToDraft(): Promise<void> {
    await this.changeStatus("draft", "Fattura riportata in bozza");
  }

  /** Send a validated invoice to SDI via PEC. */
  async sendPec(): Promise<void> {
    const inv = this.invoice();
    if (!inv || this.sending() || inv.pendingPecFilename) return;
    this.sending.set(true);
    try {
      await this.invoiceStore.sendInvoicePec(inv.id);
      const updated = await this.invoiceStore.getInvoice(inv.id);
      this.invoice.set(updated);
      this.notify.success("PEC trasmessa. Controlla la ricevuta SdI per l'esito.");
    } catch (e) {
      const updated = await this.invoiceStore.getInvoice(inv.id).catch(() => null);
      if (updated) this.invoice.set(updated);
      this.notify.error(e instanceof Error ? e.message : "Errore durante l'invio PEC");
    } finally {
      this.sending.set(false);
    }
  }

  async resolvePecAttempt(sent: boolean): Promise<void> {
    const inv = this.invoice();
    const filename = inv?.pendingPecFilename ?? inv?.recoverablePecFilename;
    if (!inv || !filename || this.sending()) return;
    this.sending.set(true);
    try {
      const confirmed = await ask(
        sent
          ? `Confermi di aver verificato nella tua casella PEC che il messaggio con ${filename} è stato accettato?`
          : `Confermi di aver verificato nella tua casella PEC che il messaggio con ${filename} non è partito? Potrai inviarlo di nuovo.`,
        { title: "Verifica tentativo PEC" },
      );
      if (!confirmed) return;
      await this.invoiceStore.resolvePecAttempt(inv.id, filename, sent);
      this.invoice.set(await this.invoiceStore.getInvoice(inv.id));
    } catch (error) {
      this.notify.error(error instanceof Error ? error.message : "Verifica PEC non registrata");
    } finally {
      this.sending.set(false);
    }
  }

  private async changeStatus(status: InvoiceStatus, successMsg: string): Promise<void> {
    const inv = this.invoice();
    if (!inv) return;
    try {
      const updated = await this.invoiceStore.setStatus(inv.id, status);
      this.invoice.set(updated);
      this.notify.success(successMsg);
    } catch (e) {
      this.notify.error(
        e instanceof Error ? e.message : "Errore durante il cambio di stato"
      );
    }
  }

  async exportXml(): Promise<void> {
    const inv = this.invoice();
    if (!inv) return;
    try {
      const xml = await this.invoiceStore.exportXml(inv.id);
      this.fileDownload.downloadTextFile(xml, `${inv.numero}.xml`, "application/xml");
      this.notify.success("XML esportato");
    } catch {
      this.notify.error("Errore durante l'esportazione XML");
    }
  }

  async downloadPdf(): Promise<void> {
    const inv = this.invoice();
    if (!inv) return;
    this.downloadingPdf.set(true);
    try {
      const pdfBytes = await this.invoiceStore.renderPdf(inv.id);
      this.fileDownload.downloadBinaryFile(
        pdfBytes,
        `Fattura_${inv.numero.replace(/\//g, "-")}.pdf`,
        "application/pdf"
      );
      this.notify.success("PDF di cortesia scaricato con successo");
    } catch {
      this.notify.error("Errore durante la generazione del PDF");
    } finally {
      this.downloadingPdf.set(false);
    }
  }

  formatBytes(bytes: number): string {
    if (!bytes || bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  }

  private base64ToUint8Array(base64: string): Uint8Array {
    const cleanBase64 = base64.replace(/\s/g, "");
    const binaryString = window.atob(cleanBase64);
    const len = binaryString.length;
    const bytes = new Uint8Array(len);
    for (let i = 0; i < len; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes;
  }

  private getMimeType(filename: string, formato?: string | null): string {
    const ext = (formato || filename.split(".").pop() || "").toLowerCase();
    switch (ext) {
      case "pdf": return "application/pdf";
      case "png": return "image/png";
      case "jpg":
      case "jpeg": return "image/jpeg";
      case "xml": return "application/xml";
      case "txt": return "text/plain";
      case "zip": return "application/zip";
      default: return "application/octet-stream";
    }
  }

  downloadAttachment(att: AttachmentDetail): void {
    try {
      const bytes = this.base64ToUint8Array(att.attachmentBase64);
      const mime = this.getMimeType(att.nomeAttachment, att.formatoAttachment);
      this.fileDownload.downloadBinaryFile(bytes, att.nomeAttachment, mime);
      this.notify.success(`Allegato "${att.nomeAttachment}" scaricato con successo`);
    } catch {
      this.notify.error("Errore durante il download dell'allegato");
    }
  }

  previewAttachment(att: AttachmentDetail): void {
    try {
      const bytes = this.base64ToUint8Array(att.attachmentBase64);
      const mime = this.getMimeType(att.nomeAttachment, att.formatoAttachment);
      const blob = new Blob([bytes as any], { type: mime });
      const url = URL.createObjectURL(blob);
      window.open(url, "_blank");
    } catch {
      this.notify.error("Impossibile aprire l'anteprima dell'allegato");
    }
  }

  printInvoice(): void {
    window.print();
  }

  async createAutofattura(tipo?: string): Promise<void> {
    const inv = this.invoice();
    if (!inv) return;
    this.generatingAutofattura.set(true);
    try {
      const draft = await this.invoiceStore.generateAutofatturaDraft(inv.id, tipo);
      this.notify.success(`Bozza di autofattura (${draft.tipoDocumento}) generata con successo!`);
      this.router.navigate(["/invoices/new"], { state: { autofatturaDraft: draft } });
    } catch (e) {
      this.notify.error(
        typeof e === "string" ? e : "Errore durante la generazione dell'autofattura"
      );
    } finally {
      this.generatingAutofattura.set(false);
    }
  }

  toggleAdeView(): void {
    this.showAdeView.update((v) => !v);
  }

  /** Rectify a rejected invoice, returning it to draft state for correction within 5 days. */
  async rectifyRejected(): Promise<void> {
    const inv = this.invoice();
    if (!inv) return;
    this.rectifying.set(true);
    try {
      const updated = await this.invoiceStore.rectifyRejectedInvoice(inv.id);
      this.invoice.set(updated);
      this.notify.success(
        "Fattura sbloccata e riportata in bozza. Correggi gli errori segnalati e re-invia entro 5 giorni."
      );
      this.router.navigate(["/invoices", inv.id, "edit"]);
    } catch (e) {
      this.notify.error(
        typeof e === "string" ? e : "Errore durante la rettifica della fattura scartata"
      );
    } finally {
      this.rectifying.set(false);
    }
  }

  /** Import an SDI receipt XML file manually into this invoice dossier. */
  async onReceiptFileSelected(event: Event): Promise<void> {
    const input = event.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;
    const file = input.files[0];
    const reader = new FileReader();

    reader.onload = async () => {
      try {
        const xml = reader.result as string;
        await this.invoiceStore.importSdiReceipt(xml);
        const inv = this.invoice();
        if (inv) {
          const reloaded = await this.invoiceStore.getInvoice(inv.id);
          this.invoice.set(reloaded);
        }
        this.notify.success("Ricevuta SDI importata con successo nel fascicolo!");
      } catch (err) {
        this.notify.error(
          typeof err === "string" ? err : "Errore nell'importazione della ricevuta SDI"
        );
      } finally {
        input.value = "";
      }
    };
    reader.readAsText(file);
  }

  downloadReceipt(notif: SdiNotificationDetail): void {
    const filename = notif.nomeFile || `Ricevuta_${notif.tipo}_${notif.id.slice(0, 5)}.xml`;
    this.fileDownload.downloadTextFile(notif.rawXml, filename, "application/xml");
  }

  viewReceipt(notif: SdiNotificationDetail): void {
    this.selectedReceipt.set(notif);
  }

  closeReceiptModal(): void {
    this.selectedReceipt.set(null);
  }

  edit(): void {
    const inv = this.invoice();
    if (inv) {
      this.router.navigate(["/invoices", inv.id, "edit"]);
    }
  }

  goBack(): void {
    this.router.navigate(["/invoices"]);
  }
}
