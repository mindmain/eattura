import { Component, inject, OnInit, signal } from "@angular/core";
import { ActivatedRoute, Router } from "@angular/router";
import { InvoiceService } from "../../../core/services/invoice.service";
import { NotificationService } from "../../../core/services/notification.service";
import { TIPO_DOCUMENTO, MODALITA_PAGAMENTO } from "../../../core/models/sdi.model";
import type { InvoiceDetail, InvoiceStatus } from "../../../core/models/invoice.model";

@Component({
  selector: "app-invoice-detail",
  standalone: true,
  imports: [],
  templateUrl: "./invoice-detail.component.html",
  styleUrl: "./invoice-detail.component.css",
})
export class InvoiceDetailComponent implements OnInit {
  private invoiceService = inject(InvoiceService);
  private router = inject(Router);
  private route = inject(ActivatedRoute);
  private notify = inject(NotificationService);

  invoice = signal<InvoiceDetail | null>(null);
  validationResult = signal<{ valid: boolean; errors: { code: string; message: string }[] } | null>(null);
  sending = signal(false);

  async ngOnInit(): Promise<void> {
    const id = this.route.snapshot.paramMap.get("id");
    if (!id) {
      this.router.navigate(["/invoices"]);
      return;
    }
    try {
      const inv = await this.invoiceService.getInvoice(id);
      this.invoice.set(inv);
    } catch {
      this.notify.error("Errore nel caricamento della fattura");
      this.router.navigate(["/invoices"]);
    }
  }

  async validate(): Promise<void> {
    const inv = this.invoice();
    if (!inv) return;
    try {
      const result = await this.invoiceService.validateInvoice(inv.id);
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

  /** Promote a draft invoice to `validated` (the backend blocks it if SDI validation fails). */
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
    if (!inv) return;
    this.sending.set(true);
    try {
      await this.invoiceService.sendInvoicePec(inv.id);
      const updated = await this.invoiceService.getInvoice(inv.id);
      this.invoice.set(updated);
      this.notify.success("Fattura inviata a SDI via PEC");
    } catch (e) {
      this.notify.error(typeof e === "string" ? e : "Errore durante l'invio PEC");
    } finally {
      this.sending.set(false);
    }
  }

  private async changeStatus(status: InvoiceStatus, successMsg: string): Promise<void> {
    const inv = this.invoice();
    if (!inv) return;
    try {
      const updated = await this.invoiceService.setStatus(inv.id, status);
      this.invoice.set(updated);
      this.notify.success(successMsg);
    } catch (e) {
      this.notify.error(
        typeof e === "string" ? e : "Errore durante il cambio di stato"
      );
    }
  }

  async exportXml(): Promise<void> {
    const inv = this.invoice();
    if (!inv) return;
    try {
      const xml = await this.invoiceService.exportXml(inv.id);
      // Download as file
      const blob = new Blob([xml], { type: "application/xml" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${inv.numero}.xml`;
      a.click();
      URL.revokeObjectURL(url);
      this.notify.success("XML esportato");
    } catch {
      this.notify.error("Errore durante l'esportazione XML");
    }
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

  tipoLabel(code: string): string {
    return TIPO_DOCUMENTO[code] ?? code;
  }

  modalitaLabel(code: string): string {
    return MODALITA_PAGAMENTO[code] ?? code;
  }

  statusLabel(status: InvoiceStatus): string {
    const labels: Record<InvoiceStatus, string> = {
      draft: "Bozza",
      validated: "Validata",
      sent: "Inviata",
      accepted: "Accettata",
      rejected: "Rifiutata",
    };
    return labels[status] ?? status;
  }

  badgeClass(status: InvoiceStatus): string {
    return `badge badge-${status}`;
  }

  formatCurrency(value: number | null): string {
    if (value == null) return "-";
    return new Intl.NumberFormat("it-IT", { style: "currency", currency: "EUR" }).format(value);
  }
}
