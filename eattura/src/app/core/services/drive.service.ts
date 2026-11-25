import { Injectable, inject, signal } from "@angular/core";
import { isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { TauriService } from "./tauri.service";
import { InvoiceStore } from "../stores/invoice.store";
import { ClientStore } from "../stores/client.store";

export interface DriveStatus {
  connected: boolean;
  accountEmail: string | null;
  backupName: string | null;
  lastBackupAt: string | null;
  lastCheckedAt: number | null;
  lastAction: string | null;
}

export interface DriveSyncReport {
  action: "uploaded" | "downloaded" | "merged" | "up_to_date" | string;
  modifiedTime: string | null;
  invoices: number;
  clients: number;
  notifications: number;
}

const DISCONNECTED_STATUS: DriveStatus = {
  connected: false,
  accountEmail: null,
  backupName: null,
  lastBackupAt: null,
  lastCheckedAt: null,
  lastAction: null,
};

@Injectable({ providedIn: "root" })
export class DriveService {
  private readonly tauri = inject(TauriService);
  private readonly invoices = inject(InvoiceStore);
  private readonly clients = inject(ClientStore);

  readonly desktop = isTauri();
  private readonly state = signal<DriveStatus>(DISCONNECTED_STATUS);
  private readonly running = signal(false);
  private readonly problem = signal<string | null>(null);
  private readonly report = signal<DriveSyncReport | null>(null);
  private unlisten?: UnlistenFn;
  private listening = false;

  readonly status = this.state.asReadonly();
  readonly busy = this.running.asReadonly();
  readonly error = this.problem.asReadonly();
  readonly lastSync = this.report.asReadonly();

  async start(): Promise<void> {
    if (!this.desktop) return;
    if (!this.listening) {
      this.listening = true;
      try {
        this.unlisten = await listen<DriveSyncReport>("drive-backup-synced", ({ payload }) => {
          void this.handleSyncEvent(payload);
        });
      } catch {
        this.listening = false;
      }
    }
    await this.run(async () => {
      await this.refresh();
    });
  }

  stop(): void {
    this.listening = false;
    this.unlisten?.();
    this.unlisten = undefined;
  }

  async connect(file: File): Promise<void> {
    await this.run(async () => {
      if (file.size > 16 * 1024) {
        throw new Error("Il file delle credenziali Google è troppo grande.");
      }
      const credentialsJson = await file.text();
      const status = await this.tauri.invoke<DriveStatus>("connect_drive", { credentialsJson });
      this.state.set(status);
      if (status.lastAction === "downloaded" || status.lastAction === "merged") {
        await Promise.all([this.invoices.refresh(), this.clients.refresh()]);
      }
    });
  }

  async syncNow(): Promise<void> {
    await this.run(async () => {
      const report = await this.tauri.invoke<DriveSyncReport | null>("sync_drive");
      await this.applyReport(report);
      await this.refresh();
    });
  }

  async disconnect(): Promise<void> {
    await this.run(async () => {
      const status = await this.tauri.invoke<DriveStatus>("disconnect_drive");
      this.state.set(status);
      this.report.set(null);
    });
  }

  private async refresh(): Promise<void> {
    const status = await this.tauri.invoke<DriveStatus>("get_drive_status");
    this.state.set(status);
  }

  private async handleSyncEvent(report: DriveSyncReport): Promise<void> {
    await this.applyReport(report);
    try {
      await this.refresh();
    } catch {
      // Ignore transient status refresh errors during background notifications.
    }
  }

  private async applyReport(report: DriveSyncReport | null): Promise<void> {
    this.report.set(report);
    if (report && (report.action === "downloaded" || report.action === "merged")) {
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
      this.problem.set(
        error instanceof Error ? error.message : "Operazione Google Drive non riuscita.",
      );
    } finally {
      this.running.set(false);
    }
  }
}
