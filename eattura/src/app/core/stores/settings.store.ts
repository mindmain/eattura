import { Injectable, computed, inject, signal } from "@angular/core";
import { TauriService } from "../services/tauri.service";
import type { ClientSummary } from "../models/client.model";

export interface CompanySettings {
  denominazione: string | null;
  nome: string | null;
  cognome: string | null;
  id_paese: string;
  id_codice: string;
  codice_fiscale: string | null;
  regime_fiscale: string;
  indirizzo: string;
  numero_civico: string | null;
  cap: string;
  comune: string;
  provincia: string | null;
  nazione: string;
  telefono: string | null;
  email: string | null;
  pec: string | null;
}

export interface PecSettings {
  email: string;
  sdi_address?: string | null;
  imap_host: string;
  imap_port: number;
  smtp_host: string;
  smtp_port: number;
}

export interface AppSettings {
  company: CompanySettings | null;
  pec: PecSettings | null;
}

@Injectable({ providedIn: "root" })
export class SettingsStore {
  private tauri = inject(TauriService);

  private _settings = signal<AppSettings | null>(null);
  private _dataDir = signal<string>("");
  private _loading = signal<boolean>(false);

  readonly settings = this._settings.asReadonly();
  readonly dataDir = this._dataDir.asReadonly();
  readonly loading = this._loading.asReadonly();

  readonly company = computed(() => this._settings()?.company ?? null);
  readonly pec = computed(() => this._settings()?.pec ?? null);
  readonly isCompanyConfigured = computed(() => !!this._settings()?.company?.id_codice);

  async load(force = false): Promise<AppSettings | null> {
    if (this._settings() !== null && !force) {
      return this._settings();
    }
    this._loading.set(true);
    try {
      const [res, dir] = await Promise.all([
        this.tauri.invoke<AppSettings>("get_settings").catch(() => null),
        this.tauri.invoke<string>("get_data_directory").catch(() => ""),
      ]);
      this._settings.set(res ?? { company: null, pec: null });
      if (dir) {
        this._dataDir.set(dir);
      }
      return this._settings();
    } finally {
      this._loading.set(false);
    }
  }

  async saveCompany(company: CompanySettings): Promise<void> {
    const current = this._settings();
    const updated = await this.tauri.invoke<AppSettings>("update_settings", {
      settings: {
        company,
        pec: current?.pec ?? null,
      },
    });
    this._settings.set(updated);
  }

  async savePec(pec: PecSettings, password?: string): Promise<void> {
    if (password && pec.email) {
      await this.tauri.invoke("set_pec_password", {
        email: pec.email,
        password,
      });
    }
    const current = this._settings();
    const updated = await this.tauri.invoke<AppSettings>("update_settings", {
      settings: {
        company: current?.company ?? null,
        pec,
      },
    });
    this._settings.set(updated);
  }

  async openDataDir(): Promise<void> {
    await this.tauri.invoke("open_data_directory");
  }

  /**
   * Helper to find which Client in the ClientStore corresponds to the configured company.
   */
  findCompanyClientId(clients: ClientSummary[]): string | null {
    const comp = this.company();
    if (!comp) return null;
    const match = clients.find(
      (c) =>
        c.idPaese?.toUpperCase() === comp.id_paese.toUpperCase() &&
        c.idCodice === comp.id_codice
    );
    return match ? match.id : null;
  }
}
