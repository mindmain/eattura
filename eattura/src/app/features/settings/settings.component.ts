import { ChangeDetectionStrategy, Component, computed, inject, OnInit } from "@angular/core";
import { FormBuilder, ReactiveFormsModule, Validators } from "@angular/forms";
import { ActivatedRoute, convertToParamMap } from "@angular/router";
import { toSignal } from "@angular/core/rxjs-interop";
import { of } from "rxjs";
import { SettingsStore } from "../../core/stores/settings.store";
import { NotificationService } from "../../core/services/notification.service";
import { SyncService } from "../../core/services/sync.service";
import type { SyncConfig, SyncStatusDto } from "../../core/models/sync.model";
import { signal } from "@angular/core";
import { REGIME_FISCALE } from "../../core/models/sdi.model";
import { ThemeService, type ThemePreference } from "../../core/services/theme.service";
import { LucideAngularModule, Monitor, Moon, Sun } from "lucide-angular";

@Component({
  selector: "app-settings",
  standalone: true,
  imports: [ReactiveFormsModule, LucideAngularModule],
  templateUrl: "./settings.component.html",
  styleUrl: "./settings.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SettingsComponent implements OnInit {
  private fb = inject(FormBuilder);
  private readonly route = inject(ActivatedRoute, { optional: true });
  private readonly routeParams = toSignal(this.route?.queryParamMap ?? of(convertToParamMap({})));
  readonly section = computed(() => this.routeParams()?.get("section") === "account" ? "account" : "app");
  readonly settingsStore = inject(SettingsStore);
  private notify = inject(NotificationService);
  readonly theme = inject(ThemeService);
  readonly themeIcons = { Monitor, Sun, Moon };

  regimiFiscali = Object.entries(REGIME_FISCALE);
  readonly dataDir = this.settingsStore.dataDir;
  private syncService = inject(SyncService);

  syncStatus = signal<SyncStatusDto | null>(null);
  syncing = signal<boolean>(false);
  lastSyncResult = signal<string | null>(null);
  subjectType = signal<"azienda" | "persona">("azienda");

  syncForm = this.fb.group({
    relayUrl: ["http://localhost:8080", Validators.required],
    passphrase: ["", Validators.required],
    deviceId: ["Desktop", Validators.required],
    apiKey: [""],
  });

  companyForm = this.fb.group({
    denominazione: [""],
    nome: [""],
    cognome: [""],
    idPaese: ["IT", Validators.required],
    idCodice: ["", [Validators.required, Validators.pattern(/^[0-9]{11}$/)]],
    codiceFiscale: [""],
    regimeFiscale: ["RF01"],
    indirizzo: ["", Validators.required],
    numeroCivico: [""],
    cap: ["", [Validators.required, Validators.pattern(/^[0-9]{5}$/)]],
    comune: ["", Validators.required],
    provincia: ["", Validators.maxLength(2)],
    nazione: ["IT", Validators.required],
    telefono: [""],
    email: ["", Validators.email],
    pec: ["", Validators.email],
  });

  pecForm = this.fb.group({
    pecEmail: ["", Validators.email],
    sdiAddress: ["sdi01@pec.fatturapa.it", [Validators.required, Validators.email]],
    imapHost: [""],
    imapPort: [993],
    smtpHost: [""],
    smtpPort: [465],
    pecPassword: [""],
  });

  constructor() {
    this.onSubjectTypeChange("azienda");
  }

  setTheme(preference: ThemePreference): void {
    this.theme.setPreference(preference);
  }

  async ngOnInit(): Promise<void> {
    const syncCfg = this.syncService.getStoredConfig();
    this.syncForm.patchValue({
      relayUrl: syncCfg.relayUrl,
      passphrase: syncCfg.passphrase,
      deviceId: syncCfg.deviceId,
      apiKey: syncCfg.apiKey ?? "",
    });
    try {
      const s = await this.settingsStore.load();
      if (s?.company) {
        const c = s.company;
        this.onSubjectTypeChange(c.nome || c.cognome ? "persona" : "azienda");
        this.companyForm.patchValue({
          denominazione: c.denominazione ?? "",
          nome: c.nome ?? "",
          cognome: c.cognome ?? "",
          idPaese: c.id_paese ?? "IT",
          idCodice: c.id_codice ?? "",
          codiceFiscale: c.codice_fiscale ?? "",
          regimeFiscale: c.regime_fiscale ?? "RF01",
          indirizzo: c.indirizzo ?? "",
          numeroCivico: c.numero_civico ?? "",
          cap: c.cap ?? "",
          comune: c.comune ?? "",
          provincia: c.provincia ?? "",
          nazione: c.nazione ?? "IT",
          telefono: c.telefono ?? "",
          email: c.email ?? "",
          pec: c.pec ?? "",
        });
      }
      if (s?.pec) {
        const p = s.pec;
        this.pecForm.patchValue({
          pecEmail: p.email ?? "",
          sdiAddress: p.sdi_address || "sdi01@pec.fatturapa.it",
          imapHost: p.imap_host ?? "",
          imapPort: p.imap_port ?? 993,
          smtpHost: p.smtp_host ?? "",
          smtpPort: p.smtp_port ?? 465,
        });
      }
    } catch {
      // Non-fatal if settings not yet initialized
    }
  }

  onSubjectTypeChange(type: "azienda" | "persona"): void {
    this.subjectType.set(type);
    for (const field of ["denominazione", "nome", "cognome"] as const) {
      const required = type === "azienda" ? field === "denominazione" : field !== "denominazione";
      const control = this.companyForm.controls[field];
      control.setValidators(required ? [Validators.required, Validators.pattern(/\S/)] : []);
      control.updateValueAndValidity();
    }
  }

  async openDataDir(): Promise<void> {
    try {
      await this.settingsStore.openDataDir();
    } catch (e) {
      console.error("openDataDir failed:", e);
      this.notify.error("Impossibile aprire la cartella archivio");
    }
  }

  async saveCompany(): Promise<void> {
    if (this.companyForm.invalid) {
      this.companyForm.markAllAsTouched();
      this.notify.error("Compila i campi obbligatori dei dati del cedente");
      return;
    }
    const v = this.companyForm.getRawValue();
    try {
      await this.settingsStore.saveCompany({
        denominazione: this.subjectType() === "azienda" ? v.denominazione?.trim() || null : null,
        nome: this.subjectType() === "persona" ? v.nome?.trim() || null : null,
        cognome: this.subjectType() === "persona" ? v.cognome?.trim() || null : null,
        id_paese: v.idPaese!,
        id_codice: v.idCodice!,
        codice_fiscale: v.codiceFiscale || null,
        regime_fiscale: v.regimeFiscale || "RF01",
        indirizzo: v.indirizzo!,
        numero_civico: v.numeroCivico || null,
        cap: v.cap!,
        comune: v.comune!,
        provincia: v.provincia || null,
        nazione: v.nazione!,
        telefono: v.telefono || null,
        email: v.email || null,
        pec: v.pec || null,
      });
      this.notify.success("Dati del cedente salvati");
    } catch (e) {
      console.error("saveCompany failed:", e);
      this.notify.error(
        e instanceof Error ? e.message : "Errore durante il salvataggio dei dati del cedente"
      );
    }
  }

  async savePec(): Promise<void> {
    if (this.pecForm.invalid) {
      this.pecForm.markAllAsTouched();
      return;
    }
    const v = this.pecForm.getRawValue();
    try {
      await this.settingsStore.savePec(
        {
          email: v.pecEmail ?? "",
          sdi_address: v.sdiAddress?.trim() || null,
          imap_host: v.imapHost ?? "",
          imap_port: Number(v.imapPort) || 993,
          smtp_host: v.smtpHost ?? "",
          smtp_port: Number(v.smtpPort) || 465,
        },
        v.pecPassword || undefined
      );
      this.pecForm.patchValue({ pecPassword: "" });
      this.notify.success("Configurazione PEC salvata");
    } catch (e) {
      console.error("savePec failed:", e);
      this.notify.error(
        e instanceof Error ? e.message : "Errore durante il salvataggio della configurazione PEC"
      );
    }
  }

  async generatePairingCode(): Promise<void> {
    try {
      const code = await this.syncService.generatePairingCode();
      this.syncForm.patchValue({ passphrase: code });
      this.notify.success("Nuovo codice di accoppiamento generato");
    } catch (e) {
      this.notify.error("Impossibile generare codice");
    }
  }

  private getSyncConfig(): SyncConfig {
    const v = this.syncForm.getRawValue();
    return {
      relayUrl: v.relayUrl?.trim() || "http://localhost:8080",
      passphrase: v.passphrase?.trim() || "",
      deviceId: v.deviceId?.trim() || "Desktop",
      apiKey: v.apiKey?.trim() || undefined,
    };
  }

  async checkRelay(): Promise<void> {
    const cfg = this.getSyncConfig();
    if (!cfg.passphrase) {
      this.notify.error("Inserisci o genera una passphrase di sincronizzazione");
      return;
    }
    this.syncing.set(true);
    try {
      const status = await this.syncService.checkRelay(cfg);
      this.syncStatus.set(status);
      if (status.serverHealthy) {
        this.notify.success(`Relay attivo: ${status.pendingMessages} pacchetti in attesa`);
      } else {
        this.notify.error("Relay non raggiungibile");
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      this.notify.error(`Errore verifica relay: ${msg}`);
    } finally {
      this.syncing.set(false);
    }
  }

  async syncAll(): Promise<void> {
    const cfg = this.getSyncConfig();
    if (!cfg.passphrase) {
      this.notify.error("Inserisci o genera una passphrase di sincronizzazione");
      return;
    }
    this.syncing.set(true);
    this.lastSyncResult.set("Sincronizzazione E2EE bidirezionale in corso...");
    try {
      const res = await this.syncService.syncAll(cfg);
      const summary = `Inviati: ${res.push.invoicesPushed} fatture, ${res.push.clientsPushed} clienti. Ricevuti: ${res.pull.invoicesProcessed} fatture, ${res.pull.clientsProcessed} clienti (${res.pull.envelopesReceived} buste elaborate).`;
      this.lastSyncResult.set(summary);
      this.notify.success("Sincronizzazione completata con successo!");
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      this.lastSyncResult.set(`Errore sincronizzazione: ${msg}`);
      this.notify.error(`Errore sincronizzazione: ${msg}`);
    } finally {
      this.syncing.set(false);
    }
  }

  async pushSync(): Promise<void> {
    const cfg = this.getSyncConfig();
    if (!cfg.passphrase) {
      this.notify.error("Inserisci una passphrase di sincronizzazione");
      return;
    }
    this.syncing.set(true);
    try {
      const res = await this.syncService.pushChanges(cfg);
      this.lastSyncResult.set(`Inviati con successo ${res.invoicesPushed} fatture e ${res.clientsPushed} clienti cifrati in E2EE.`);
      this.notify.success("Modifiche inviate al relay con successo!");
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      this.notify.error(`Errore invio relay: ${msg}`);
    } finally {
      this.syncing.set(false);
    }
  }

  async pullSync(): Promise<void> {
    const cfg = this.getSyncConfig();
    if (!cfg.passphrase) {
      this.notify.error("Inserisci una passphrase di sincronizzazione");
      return;
    }
    this.syncing.set(true);
    try {
      const res = await this.syncService.pullChanges(cfg);
      this.lastSyncResult.set(`Ricevuti: ${res.invoicesProcessed} fatture, ${res.clientsProcessed} clienti da ${res.envelopesReceived} buste.`);
      this.notify.success(`Sincronizzazione ricevuta: ${res.invoicesProcessed} fatture, ${res.clientsProcessed} clienti.`);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      this.notify.error(`Errore ricezione relay: ${msg}`);
    } finally {
      this.syncing.set(false);
    }
  }
}
