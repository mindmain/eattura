import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  input,
  OnInit,
  signal,
} from "@angular/core";
import { Router, RouterLink } from "@angular/router";
import { InvoiceStore } from "../../../core/stores/invoice.store";
import { NotificationService } from "../../../core/services/notification.service";
import { FileDialogService } from "../../../core/services/file-dialog.service";
import { TipoDocumentoPipe } from "../../../shared/pipes/tipo-documento.pipe";
import { InvoiceStatusPipe } from "../../../shared/pipes/invoice-status.pipe";
import { AppCurrencyPipe } from "../../../shared/pipes/app-currency.pipe";
import type { InvoiceSummary } from "../../../core/models/invoice.model";

const MONTH_NAMES = [
  "Gennaio", "Febbraio", "Marzo", "Aprile", "Maggio", "Giugno",
  "Luglio", "Agosto", "Settembre", "Ottobre", "Novembre", "Dicembre",
];

@Component({
  selector: "app-invoice-list",
  standalone: true,
  imports: [RouterLink, TipoDocumentoPipe, InvoiceStatusPipe, AppCurrencyPipe],
  templateUrl: "./invoice-list.component.html",
  styleUrl: "./invoice-list.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class InvoiceListComponent implements OnInit {
  readonly invoiceStore = inject(InvoiceStore);
  private fileDialog = inject(FileDialogService);
  private router = inject(Router);
  private notify = inject(NotificationService);

  /** Router query param inputs bound via withComponentInputBinding */
  year = input<string>("");
  month = input<string>("");

  readonly invoices = this.invoiceStore.invoices;
  readonly loading = this.invoiceStore.loading;
  readonly pecSyncStatus = this.invoiceStore.pecSyncStatus;
  readonly secondsToPecSync = this.invoiceStore.secondsToPecSync;
  readonly pecCountdown = computed(() => {
    const seconds = this.secondsToPecSync();
    if (seconds === null) return "";
    return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
  });

  searchTerm = signal("");
  statusFilter = signal<string>("all");
  syncing = signal(false);
  importing = signal(false);

  /** Internal filter signals populated from inputs or user navigation */
  yearFilter = signal<string>("");
  monthFilter = signal<string>("");

  constructor() {
    effect(() => {
      this.yearFilter.set(this.year() ?? "");
      this.monthFilter.set(this.month() ?? "");
    });
  }

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
    const term = this.searchTerm().trim().toLowerCase();
    if (term) {
      list = list.filter((i) => {
        const text = i.searchText ?? [i.numero, i.cedenteDenominazione, i.cessionarioDenominazione]
          .join(" ").toLowerCase();
        return term.split(/\s+/).every((part) => text.includes(part));
      });
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
    { label: "Importate", value: "imported" },
  ];

  async ngOnInit(): Promise<void> {
    try {
      await this.invoiceStore.loadAll();
    } catch {
      this.notify.error("Errore nel caricamento delle fatture");
    }
  }

  /** Clear the active period filter and drop it from the URL. */
  clearPeriod(): void {
    this.yearFilter.set("");
    this.monthFilter.set("");
    this.router.navigate([], { queryParams: {} });
  }

  onSearch(event: Event): void {
    const input = event.target as HTMLInputElement;
    this.searchTerm.set(input.value);
  }

  setStatusFilter(status: string): void {
    this.statusFilter.set(status);
  }

  formatInvoiceDate(date: string): string {
    return date.replace(/^(\d{4})-(\d{2})-(\d{2})$/, "$3/$2/$1");
  }

  navigateToDetail(id: string): void {
    this.router.navigate(["/invoices", id]);
  }

  navigateToEdit(id: string): void {
    this.router.navigate(["/invoices", id, "edit"]);
  }

  /** Import a single XML invoice file via isolated file dialog service. */
  async importXml(): Promise<void> {
    try {
      const fileData = await this.fileDialog.pickAndReadXmlFile();
      if (!fileData) {
        return; // user cancelled
      }
      const detail = await this.invoiceStore.importXml(fileData.content);
      this.notify.success(`Fattura ${detail.numero} importata`);
    } catch (e) {
      this.notify.error(
        typeof e === "string" ? e : "Errore durante l'import del file XML"
      );
    }
  }

  /** Select a folder and recursively import every .xml invoice found. */
  async importFolder(): Promise<void> {
    try {
      const selectedDir = await this.fileDialog.pickDirectory();
      if (!selectedDir) {
        return; // user cancelled
      }
      this.importing.set(true);
      const summary = await this.invoiceStore.importFromDir(selectedDir);
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
      const results = await this.invoiceStore.syncPec();
      const invoices = results.filter((r) => r.tipo === "FATTURA").length;
      const notifications = results.length - invoices;
      const updated = results.filter((r) => r.newStatus).length;
      this.notify.success(
        `Sincronizzazione PEC completata: ${invoices} fatture, ${notifications} notifiche, ${updated} stati aggiornati`
      );
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
    if (typeof window !== "undefined" && !window.confirm(`Eliminare la fattura ${inv.numero}? L'operazione è irreversibile.`)) {
      return;
    }
    try {
      await this.invoiceStore.deleteInvoice(inv.id);
      this.notify.success("Fattura eliminata");
    } catch (e) {
      this.notify.error(
        typeof e === "string" ? e : "Errore durante l'eliminazione della fattura"
      );
    }
  }

  /** Signed amount: negative for purchases (costs), positive for sales; inverted for credit notes (TD04, TD08). */
  calcSignedAmount(inv: InvoiceSummary): number | null {
    const value = inv.importoTotale;
    if (value == null) return null;
    const isCreditNote = inv.tipoDocumento === "TD04" || inv.tipoDocumento === "TD08";
    const signed = inv.direction === "purchase" ? -value : value;
    return isCreditNote ? -signed : signed;
  }
}
