import { Component, inject, OnInit } from "@angular/core";
import { FormBuilder, ReactiveFormsModule, Validators } from "@angular/forms";
import { TauriService } from "../../core/services/tauri.service";
import { NotificationService } from "../../core/services/notification.service";
import { REGIME_FISCALE } from "../../core/models/sdi.model";

@Component({
  selector: "app-settings",
  standalone: true,
  imports: [ReactiveFormsModule],
  templateUrl: "./settings.component.html",
  styleUrl: "./settings.component.css",
})
export class SettingsComponent implements OnInit {
  private fb = inject(FormBuilder);
  private tauri = inject(TauriService);
  private notify = inject(NotificationService);

  regimiFiscali = Object.entries(REGIME_FISCALE);

  companyForm = this.fb.group({
    denominazione: [""],
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
    codiceDestinatario: ["0000000"],
  });

  pecForm = this.fb.group({
    pecEmail: ["", Validators.email],
    imapHost: [""],
    imapPort: [993],
    smtpHost: [""],
    smtpPort: [465],
    pecPassword: [""],
  });

  async ngOnInit(): Promise<void> {
    try {
      const settings = await this.tauri.invoke<Record<string, any>>("get_settings");
      if (settings?.['company']) {
        const c = settings['company'];
        this.companyForm.patchValue({
          denominazione: c.denominazione ?? "",
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
      if (settings?.['pec']) {
        const p = settings['pec'];
        this.pecForm.patchValue({
          pecEmail: p.email ?? "",
          imapHost: p.imap_host ?? "",
          imapPort: p.imap_port ?? 993,
          smtpHost: p.smtp_host ?? "",
          smtpPort: p.smtp_port ?? 465,
        });
      }
    } catch {
      // Settings may not exist yet, that is fine
    }
  }

  async saveCompany(): Promise<void> {
    if (this.companyForm.invalid) {
      this.companyForm.markAllAsTouched();
      this.notify.error("Compila i campi obbligatori dei dati azienda");
      return;
    }
    const v = this.companyForm.getRawValue();
    try {
      // Build the company object in the snake_case shape the backend expects,
      // preserving any existing PEC settings.
      const current = await this.tauri.invoke<Record<string, any>>("get_settings");
      await this.tauri.invoke("update_settings", {
        settings: {
          company: {
            denominazione: v.denominazione || null,
            nome: null,
            cognome: null,
            id_paese: v.idPaese,
            id_codice: v.idCodice,
            codice_fiscale: v.codiceFiscale || null,
            regime_fiscale: v.regimeFiscale,
            indirizzo: v.indirizzo,
            numero_civico: v.numeroCivico || null,
            cap: v.cap,
            comune: v.comune,
            provincia: v.provincia || null,
            nazione: v.nazione,
            telefono: v.telefono || null,
            email: v.email || null,
            pec: v.pec || null,
          },
          pec: current?.["pec"] ?? null,
        },
      });
      this.notify.success("Dati azienda salvati");
    } catch (e) {
      console.error("update_settings (company) failed:", e);
      this.notify.error(
        e instanceof Error ? e.message : "Errore durante il salvataggio dei dati azienda"
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
      // Store the password securely in the OS keyring (never in the DB).
      if (v.pecEmail && v.pecPassword) {
        await this.tauri.invoke("set_pec_password", {
          email: v.pecEmail,
          password: v.pecPassword,
        });
      }
      // Persist non-secret PEC config in the shape the backend expects,
      // preserving any existing company settings.
      const current = await this.tauri.invoke<Record<string, any>>("get_settings");
      await this.tauri.invoke("update_settings", {
        settings: {
          company: current?.["company"] ?? null,
          pec: {
            email: v.pecEmail,
            imap_host: v.imapHost,
            imap_port: v.imapPort,
            smtp_host: v.smtpHost,
            smtp_port: v.smtpPort,
          },
        },
      });
      this.pecForm.patchValue({ pecPassword: "" });
      this.notify.success("Configurazione PEC salvata");
    } catch (e) {
      console.error("savePec failed:", e);
      this.notify.error(
        e instanceof Error ? e.message : "Errore durante il salvataggio della configurazione PEC"
      );
    }
  }
}
