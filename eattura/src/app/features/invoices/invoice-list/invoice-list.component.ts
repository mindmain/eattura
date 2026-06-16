import { Component, computed, inject, OnInit, signal } from "@angular/core";
import { ActivatedRoute, Router, RouterLink } from "@angular/router";
import { open } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";
import { InvoiceService } from "../../../core/services/invoice.service";
import { NotificationService } from "../../../core/services/notification.service";
import { TIPO_DOCUMENTO } from "../../../core/models/sdi.model";
import type { InvoiceSummary, InvoiceStatus } from "../../../core/models/invoice.model";

/** Italian month names, January first (for the period filter label). */
const MONTH_NAMES = [
  "Gennaio", "Febbraio", "Marzo", "Aprile", "Maggio", "Giugno",
  "Luglio", "Agosto", "Settembre", "Ottobre", "Novembre", "Dicembre",
];

@Component({
  selector: "app-invoice-list",
  standalone: true,
  imports: [RouterLink],
  templateUrl: "./invoice-list.component.html",
  styleUrl: "./invoice-list.component.css",
})
export class InvoiceListComponent implements OnInit {
  private invoiceService = inject(InvoiceService);
  private router = inject(Router);
  private route = inject(ActivatedRoute);
  private notify = inject(NotificationService);

  invoices = signal<InvoiceSummary[]>([]);
  searchTerm = signal("");
  statusFilter = signal<string>("all");
  syncing = signal(false);
  importing = signal(false);

  /** Period filter from the dashboard (year, and optionally month "01".."12"). */
  yearFilter = signal<string>("");
  monthFilter = signal<string>("");

  /** Human-readable label of the active period filter, or empty when none. */
  periodLabel = computed(() => {
    const year = this.yearFilter();
    if (!year) return "";
    const month = this.monthFilter();
    return month ? `${MONTH_NAMES[Number(month) - 1] ?? month} ${year}` : year;
  });

  filteredInvoices = computed(() => {
    let list = this.invoices();
    const status = this.statusFilter();
    if (status !== "all") {
      list = list.filter((i) => i.stato === status);
    }
    // Period filter: match the invoice date prefix (year or year-month).
    const year = this.yearFilter();
    if (year) {
      const prefix = this.monthFilter() ? `${year}-${this.monthFilter()}` : year;
      list = list.filter((i) => i.data?.startsWith(prefix));
    }
    const term = this.searchTerm().toLowerCase();
    if (term) {
      list = list.filter(
        (i) =>
          i.numero.toLowerCase().includes(term) ||
          i.cedenteDenominazione.toLowerCase().includes(term) ||
          i.cessionarioDenominazione.toLowerCase().includes(term)
      );
    }
    return list;
  });

  statusTabs: { label: string; value: string }[] = [
    { label: "Tutte", value: "all" },
    { label: "Bozza", value: "draft" },
    { label: "Validate", value: "validated" },
    { label: "Inviate", value: "sent" },
    { label: "Accettate", value: "accepted" },
    { label: "Rifiutate", value: "rejected" },
  ];

  async ngOnInit(): Promise<void> {
    // Apply a period filter passed from the dashboard via query params.
    const qp = this.route.snapshot.queryParamMap;
    this.yearFilter.set(qp.get("year") ?? "");
    this.monthFilter.set(qp.get("month") ?? "");
    await this.loadInvoices();
  }

  /** Clear the active period filter and drop it from the URL. */
  clearPeriod(): void {
    this.yearFilter.set("");
    this.monthFilter.set("");
    this.router.navigate([], { relativeTo: this.route, queryParams: {} });
  }

  onSearch(event: Event): void {
    const input = event.target as HTMLInputElement;
    this.searchTerm.set(input.value);
  }

  setStatusFilter(status: string): void {
    this.statusFilter.set(status);
  }

  navigateToDetail(id: string): void {
    this.router.navigate(["/invoices", id]);
  }

  navigateToEdit(id: string): void {
    this.router.navigate(["/invoices", id, "edit"]);
  }

  /** Import a single XML invoice file via a file picker. */
  async importXml(): Promise<void> {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Fattura XML", extensions: ["xml"] }],
      });
      if (!selected || Array.isArray(selected)) {
        return; // user cancelled
      }
      const content = await readTextFile(selected);
      const detail = await this.invoiceService.importXml(content);
      this.notify.success(`Fattura ${detail.numero} importata`);
      await this.loadInvoices();
    } catch (e) {
      this.notify.error(
        typeof e === "string" ? e : "Errore durante l'import del file XML"
      );
    }
  }

  /** Select a folder and recursively import every .xml invoice found. */
  async importFolder(): Promise<void> {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected || Array.isArray(selected)) {
        return; // user cancelled
      }
      this.importing.set(true);
      const summary = await this.invoiceService.importFromDir(selected);
      if (summary.imported > 0) {
        this.notify.success(
          `Importate ${summary.imported}/${summary.totalFiles} fatture (clienti creati automaticamente). ${summary.skipped} saltate.`
        );
      } else {
        this.notify.error(
          `Nessuna fattura importata su ${summary.totalFiles} file XML trovati (${summary.skipped} saltati).`
        );
      }
      if (summary.errors.length > 0) {
        console.warn("Import cartella — file saltati:", summary.errors);
      }
      await this.loadInvoices();
    } catch (e) {
      this.notify.error(
        typeof e === "string" ? e : "Errore durante l'import della cartella"
      );
    } finally {
      this.importing.set(false);
    }
  }

  async syncPec(): Promise<void> {
    this.syncing.set(true);
    try {
      const results = await this.invoiceService.syncPec();
      const updated = results.filter((r) => r.newStatus).length;
      this.notify.success(
        `Sincronizzazione PEC completata: ${results.length} notifiche, ${updated} fatture aggiornate`
      );
      await this.loadInvoices();
    } catch (e) {
      this.notify.error(
        typeof e === "string" ? e : "Errore durante la sincronizzazione PEC"
      );
    } finally {
      this.syncing.set(false);
    }
  }

  async deleteInvoice(inv: InvoiceSummary, event: Event): Promise<void> {
    event.stopPropagation();
    if (!confirm(`Eliminare la fattura ${inv.numero}? L'operazione è irreversibile.`)) {
      return;
    }
    try {
      await this.invoiceService.deleteInvoice(inv.id);
      this.notify.success("Fattura eliminata");
      await this.loadInvoices();
    } catch (e) {
      this.notify.error(
        typeof e === "string" ? e : "Errore durante l'eliminazione della fattura"
      );
    }
  }

  tipoLabel(code: string): string {
    return TIPO_DOCUMENTO[code] ?? code;
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

  /** Signed amount: negative for purchases (costs), positive for sales. */
  formatAmount(inv: InvoiceSummary): string {
    const value = inv.importoTotale ?? 0;
    const signed = inv.direction === "purchase" ? -value : value;
    return this.formatCurrency(signed);
  }

  /** CSS class coloring the amount by direction (income blue, cost red). */
  amountClass(inv: InvoiceSummary): string {
    if (inv.direction === "purchase") return "amount-cost";
    if (inv.direction === "sale") return "amount-income";
    return "";
  }

  private async loadInvoices(): Promise<void> {
    try {
      const list = await this.invoiceService.listInvoices();
      this.invoices.set(list);
    } catch {
      this.notify.error("Errore nel caricamento delle fatture");
    }
  }
}
