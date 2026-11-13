import { Injectable, inject, signal } from "@angular/core";
import { isTauri } from "@tauri-apps/api/core";
import { TauriService } from "./tauri.service";
import { InvoiceStore } from "../stores/invoice.store";
import { ClientStore } from "../stores/client.store";

interface DriveStatus {
  folderName: string | null;
}

interface DriveSyncReport {
  imported: number;
  unchanged: number;
  ignored: number;
  failed: number;
  errors: string[];
}

@Injectable({ providedIn: "root" })
export class DriveService {
  private readonly tauri = inject(TauriService);
  private readonly invoices = inject(InvoiceStore);
  private readonly clients = inject(ClientStore);

  readonly desktop = isTauri();
  private readonly folder = signal<string | null>(null);
  private readonly running = signal(false);
  private readonly problem = signal<string | null>(null);
  private readonly report = signal<DriveSyncReport | null>(null);
  readonly folderName = this.folder.asReadonly();
  readonly busy = this.running.asReadonly();
  readonly error = this.problem.asReadonly();
  readonly lastSync = this.report.asReadonly();

  async start(): Promise<void> {
    await this.run(async () => {
      await this.refresh();
      if (this.folder()) await this.sync();
    });
  }

  async connect(file: File): Promise<void> {
    await this.run(async () => {
      if (file.size > 16 * 1024) throw new Error("Il file delle credenziali Google è troppo grande.");
      const credentialsJson = await file.text();
      const status = await this.tauri.invoke<DriveStatus>("connect_drive", { credentialsJson });
      this.folder.set(status.folderName);
      this.report.set(null);
      await this.sync();
    });
  }

  async syncNow(): Promise<void> {
    await this.run(() => this.sync());
  }

  async authorizeFiles(): Promise<void> {
    await this.run(async () => {
      const report = await this.tauri.invoke<DriveSyncReport>("authorize_drive_files");
      await this.applyReport(report);
    });
  }

  async disconnect(): Promise<void> {
    await this.run(async () => {
      const status = await this.tauri.invoke<DriveStatus>("disconnect_drive");
      this.folder.set(status.folderName);
      this.report.set(null);
    });
  }

  private async refresh(): Promise<void> {
    const status = await this.tauri.invoke<DriveStatus>("get_drive_status");
    this.folder.set(status.folderName);
  }

  private async sync(): Promise<void> {
    const report = await this.tauri.invoke<DriveSyncReport | null>("sync_drive");
    await this.applyReport(report);
  }

  private async applyReport(report: DriveSyncReport | null): Promise<void> {
    this.report.set(report);
    if (report?.imported || report?.failed) {
      await Promise.all([this.invoices.refresh(), this.clients.refresh()]);
    }
  }

  private async run(operation: () => Promise<void>): Promise<void> {
    if (!this.desktop || this.running()) return;
    this.problem.set(null);
    this.running.set(true);
    try {
      await operation();
    } catch (error) {
      this.problem.set(error instanceof Error ? error.message : "Operazione Google Drive non riuscita.");
    } finally {
      this.running.set(false);
    }
  }
}
