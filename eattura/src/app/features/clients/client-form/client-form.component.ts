import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  input,
  signal,
} from "@angular/core";
import { FormBuilder, ReactiveFormsModule, Validators } from "@angular/forms";
import { Router } from "@angular/router";
import { ClientStore } from "../../../core/stores/client.store";
import { NotificationService } from "../../../core/services/notification.service";
import { ViesService } from "../../../core/services/vies.service";
import { italianTaxIds } from "../../../core/models/italian-tax-id";
import { REGIME_FISCALE } from "../../../core/models/sdi.model";

@Component({
  selector: "app-client-form",
  standalone: true,
  imports: [ReactiveFormsModule],
  templateUrl: "./client-form.component.html",
  styleUrl: "./client-form.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ClientFormComponent {
  private fb = inject(FormBuilder);
  private clientStore = inject(ClientStore);
  private router = inject(Router);
  private notify = inject(NotificationService);
  private viesService = inject(ViesService);

  isViesLoading = signal<boolean>(false);
  viesStatus = signal<{ valid: boolean; message: string } | null>(null);

  /** Router input binding */
  id = input<string | undefined>();

  isEditMode = computed(() => !!this.id());
  subjectType = signal<"azienda" | "persona">("azienda");

  regimiFiscali = Object.entries(REGIME_FISCALE);

  form = this.fb.group({
    denominazione: ["", Validators.required],
    nome: [""],
    cognome: [""],
    idPaese: ["IT", Validators.required],
    idCodice: ["", Validators.required],
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
    codiceDestinatario: ["0000000", [Validators.pattern(/^[A-Za-z0-9]{7}$/)]],
    isPa: [false],
  }, { validators: italianTaxIds });

  constructor() {
    effect(async () => {
      const clientId = this.id();
      if (!clientId) return;

      try {
        const client = await this.clientStore.getClient(clientId);
        const isPa = !!(client.isPa || (client.codiceDestinatario && client.codiceDestinatario.length === 6));
        const isPersona = !!(client.nome || client.cognome);
        if (isPersona) {
          this.onSubjectTypeChange("persona");
        } else {
          this.onSubjectTypeChange("azienda");
        }
        const isCfOnly =
          client.idPaese.toUpperCase() === "IT" &&
          client.idCodice?.length === 16 &&
          /[A-Za-z]/.test(client.idCodice);
        this.form.patchValue({
          ...client,
          idCodice: isCfOnly ? "" : client.idCodice,
          codiceFiscale: client.codiceFiscale || (isCfOnly ? client.idCodice : ""),
          isPa,
        });
        if (this.form.hasError("invalidVat")) this.form.controls.idCodice.markAsTouched();
        if (this.form.hasError("invalidFiscalCode")) this.form.controls.codiceFiscale.markAsTouched();
        this.updateCodiceDestinatarioValidation(isPa);
      } catch {
        this.notify.error("Errore nel caricamento del cliente");
        this.router.navigate(["/clients"]);
      }
    });
  }

  async lookupVies(): Promise<void> {
    const rawCountry = (this.form.get("idPaese")?.value || "IT").trim().toUpperCase();
    const rawVat = (this.form.get("idCodice")?.value || "").trim();

    if (!rawVat) {
      this.notify.warning("Inserisci prima il numero di Partita IVA da verificare");
      return;
    }

    this.isViesLoading.set(true);
    this.viesStatus.set(null);

    try {
      const res = await this.viesService.lookupVat(rawCountry, rawVat);
      if (res.valid) {
        this.viesStatus.set({ valid: true, message: "Attiva e valida nel registro VIES" });
        this.notify.success(`Partita IVA valida! Dati recuperati per ${res.name || rawVat}`);

        const patch: Record<string, string | null> = {};
        if (res.name) {
          patch["denominazione"] = res.name;
          this.onSubjectTypeChange("azienda");
        }
        if (res.street) {
          patch["indirizzo"] = res.street;
        } else if (res.address) {
          patch["indirizzo"] = res.address;
        }
        if (res.postalCode) {
          patch["cap"] = res.postalCode;
        }
        if (res.city) {
          patch["comune"] = res.city;
        }
        if (res.province) {
          patch["provincia"] = res.province;
        }
        if (res.countryCode) {
          patch["idPaese"] = res.countryCode;
          patch["nazione"] = res.countryCode;
        }
        if (!this.form.get("codiceFiscale")?.value && (res.countryCode === "IT" || rawCountry === "IT")) {
          patch["codiceFiscale"] = res.vatNumber;
        }
        this.form.patchValue(patch);
      } else {
        const msg = res.userError || "Partita IVA inesistente o non registrata al VIES";
        this.viesStatus.set({ valid: false, message: msg });
        this.notify.warning(`Attenzione: ${msg}`);
      }
    } catch (err: unknown) {
      const errStr = err instanceof Error ? err.message : String(err);
      this.viesStatus.set({ valid: false, message: errStr });
      this.notify.error(`Impossibile verificare VIES: ${errStr}`);
    } finally {
      this.isViesLoading.set(false);
    }
  }

  onPaChange(): void {
    const isPa = !!this.form.get("isPa")?.value;
    if (isPa && this.form.get("codiceDestinatario")?.value === "0000000") {
      this.form.patchValue({ codiceDestinatario: "" });
    }
    this.updateCodiceDestinatarioValidation(isPa);
  }

  private updateCodiceDestinatarioValidation(isPa: boolean): void {
    const ctrl = this.form.get("codiceDestinatario");
    if (!ctrl) return;
    if (isPa) {
      ctrl.setValidators([Validators.required, Validators.pattern(/^[A-Za-z0-9]{6}$/)]);
    } else {
      ctrl.setValidators([Validators.pattern(/^[A-Za-z0-9]{7}$/)]);
    }
    ctrl.updateValueAndValidity();
  }

  onSubjectTypeChange(type: "azienda" | "persona"): void {
    this.subjectType.set(type);
    const idCodiceCtrl = this.form.get("idCodice");
    const denomCtrl = this.form.get("denominazione");
    const nomeCtrl = this.form.get("nome");
    const cognomeCtrl = this.form.get("cognome");
    if (type === "azienda") {
      this.form.patchValue({ nome: "", cognome: "" });
      denomCtrl?.setValidators([Validators.required]);
      nomeCtrl?.setValidators([]);
      cognomeCtrl?.setValidators([]);
      idCodiceCtrl?.setValidators(Validators.required);
    } else {
      this.form.patchValue({ denominazione: "" });
      denomCtrl?.setValidators([]);
      nomeCtrl?.setValidators([Validators.required]);
      cognomeCtrl?.setValidators([Validators.required]);
      idCodiceCtrl?.clearValidators();
    }
    denomCtrl?.updateValueAndValidity();
    nomeCtrl?.updateValueAndValidity();
    cognomeCtrl?.updateValueAndValidity();
    idCodiceCtrl?.updateValueAndValidity();
  }

  async onSubmit(): Promise<void> {
    const value = this.form.getRawValue();
    const rawVat = (value.idCodice ?? "").trim();
    const rawCf = (value.codiceFiscale ?? "").trim().toUpperCase();

    if (!rawVat && !rawCf) {
      this.form.markAllAsTouched();
      this.notify.error("Inserisci la Partita IVA o il Codice Fiscale");
      return;
    }

    if (this.form.invalid) {
      this.form.markAllAsTouched();
      return;
    }

    const request = {
      denominazione: this.subjectType() === "azienda" ? value.denominazione?.trim() || null : null,
      nome: this.subjectType() === "persona" ? value.nome?.trim() || null : null,
      cognome: this.subjectType() === "persona" ? value.cognome?.trim() || null : null,
      idPaese: value.idPaese!.trim().toUpperCase(),
      idCodice: rawVat,
      codiceFiscale: rawCf || null,
      regimeFiscale: value.regimeFiscale || null,
      indirizzo: value.indirizzo!,
      numeroCivico: value.numeroCivico || null,
      cap: value.cap!,
      comune: value.comune!,
      provincia: value.provincia || null,
      nazione: value.nazione!,
      telefono: value.telefono || null,
      email: value.email || null,
      pec: value.pec || null,
      codiceDestinatario: value.codiceDestinatario ? value.codiceDestinatario.toUpperCase() : null,
      isPa: !!value.isPa,
    };

    try {
      const clientId = this.id();
      if (this.isEditMode() && clientId) {
        await this.clientStore.updateClient(clientId, request);
        this.notify.success("Cliente aggiornato");
      } else {
        await this.clientStore.createClient(request);
        this.notify.success("Cliente creato");
      }
      this.router.navigate(["/clients"]);
    } catch {
      this.notify.error("Errore durante il salvataggio del cliente");
    }
  }

  cancel(): void {
    this.router.navigate(["/clients"]);
  }
}
