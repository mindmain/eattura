import { ChangeDetectionStrategy, Component, inject, signal } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { ExportService } from "../../core/services/export.service";
import { NotificationService } from "../../core/services/notification.service";
import type {
  ExportFilter,
  AgidPreservationVerificationResult,
} from "../../core/models/export.model";

function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result as string;
      const base64 = result.includes(",") ? result.split(",")[1] : result;
      resolve(base64);
    };
    reader.onerror = (err) => reject(err);
    reader.readAsDataURL(file);
  });
}

@Component({
  selector: "app-export",
  standalone: true,
  imports: [FormsModule],
  templateUrl: "./export.component.html",
  styleUrls: ["./export.component.scss"],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExportComponent {
  private exportService = inject(ExportService);
  private notifications = inject(NotificationService);

  readonly currentYear = new Date().getFullYear();
  years = [this.currentYear, this.currentYear - 1, this.currentYear - 2, this.currentYear - 3];

  selectedYear = signal<number>(this.currentYear);
  selectedPeriod = signal<string>("all");
  selectedDirection = signal<string>("all");
  includePdf = signal<boolean>(true);
  includeSdiReceipts = signal<boolean>(true);

  loading = signal<boolean>(false);
  lastAction = signal<string>("");
  verifying = signal<boolean>(false);
  verificationResult = signal<AgidPreservationVerificationResult | null>(null);

  private buildFilter(): ExportFilter {
    return {
      year: this.selectedYear(),
      period: this.selectedPeriod(),
      direction: this.selectedDirection(),
      includePdf: this.includePdf(),
      includeSdiReceipts: this.includeSdiReceipts(),
    };
  }

  async exportAccountantZip(): Promise<void> {
    this.loading.set(true);
    this.lastAction.set("Generazione pacchetto commercialista ZIP in corso...");
    try {
      const res = await this.exportService.exportAccountantPackage(this.buildFilter());
      this.exportService.downloadBase64(res.zipBase64, res.filename);
      this.notifications.success(
        `Pacchetto ${res.filename} generato con successo (${res.invoiceCount} fatture incluse)`
      );
    } catch (err: unknown) {
      this.notifications.error("Errore esportazione pacchetto: " + String(err));
    } finally {
      this.loading.set(false);
    }
  }

  async exportPrimaNotaCsv(): Promise<void> {
    this.loading.set(true);
    this.lastAction.set("Generazione tracciato Prima Nota CSV in corso...");
    try {
      const res = await this.exportService.exportAccountantCsv(this.buildFilter());
      this.exportService.downloadText(res.csvContent, res.filename, "text/csv;charset=utf-8;");
      this.notifications.success(
        `File ${res.filename} esportato con successo (${res.rowCount} registrazioni)`
      );
    } catch (err: unknown) {
      this.notifications.error("Errore esportazione CSV: " + String(err));
    } finally {
      this.loading.set(false);
    }
  }

  async exportAgidPackage(): Promise<void> {
    this.loading.set(true);
    this.lastAction.set("Costruzione pacchetto di versamento SIP (UNI 11386:2010 SInCRO)...");
    try {
      const res = await this.exportService.exportAgidPreservationPackage(this.buildFilter());
      this.exportService.downloadBase64(res.zipBase64, res.filename);
      this.notifications.success(
        `Pacchetto AgID generato con successo: ${res.filename} (impronte SHA-256 verificate)`
      );
    } catch (err: unknown) {
      this.notifications.error("Errore generazione pacchetto AgID: " + String(err));
    } finally {
      this.loading.set(false);
    }
  }

  async onVerificationFileSelected(event: Event): Promise<void> {
    const input = event.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;

    const file = input.files[0];
    this.verifying.set(true);
    this.verificationResult.set(null);

    try {
      const base64 = await fileToBase64(file);
      const res = await this.exportService.verifyAgidPreservationPackage(base64);
      this.verificationResult.set(res);
      if (res.valid) {
        this.notifications.success("Verifica completata: Pacchetto AgID valido e integro.");
      } else {
        this.notifications.error(`Verifica fallita: ${res.errors.length} anomalie riscontrate.`);
      }
    } catch (err: unknown) {
      this.notifications.error("Errore verifica: " + String(err));
    } finally {
      this.verifying.set(false);
      input.value = "";
    }
  }
}
