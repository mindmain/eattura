import { Component, inject, OnInit, signal } from "@angular/core";
import { FormBuilder, ReactiveFormsModule, Validators } from "@angular/forms";
import { ActivatedRoute, Router } from "@angular/router";
import { ClientService } from "../../../core/services/client.service";
import { NotificationService } from "../../../core/services/notification.service";
import { REGIME_FISCALE } from "../../../core/models/sdi.model";

@Component({
  selector: "app-client-form",
  standalone: true,
  imports: [ReactiveFormsModule],
  templateUrl: "./client-form.component.html",
  styleUrl: "./client-form.component.css",
})
export class ClientFormComponent implements OnInit {
  private fb = inject(FormBuilder);
  private clientService = inject(ClientService);
  private router = inject(Router);
  private route = inject(ActivatedRoute);
  private notify = inject(NotificationService);

  isEditMode = signal(false);
  clientId = signal<string | null>(null);
  subjectType = signal<"azienda" | "persona">("azienda");

  regimiFiscali = Object.entries(REGIME_FISCALE);

  form = this.fb.group({
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
    codiceDestinatario: ["0000000"],
  });

  async ngOnInit(): Promise<void> {
    const id = this.route.snapshot.paramMap.get("id");
    if (id) {
      this.isEditMode.set(true);
      this.clientId.set(id);
      try {
        const client = await this.clientService.getClient(id);
        this.form.patchValue(client);
        // Detect subject type from loaded data
        if (client.nome || client.cognome) {
          this.subjectType.set("persona");
        }
      } catch {
        this.notify.error("Errore nel caricamento del cliente");
        this.router.navigate(["/clients"]);
      }
    }
  }

  onSubjectTypeChange(type: "azienda" | "persona"): void {
    this.subjectType.set(type);
    if (type === "azienda") {
      this.form.patchValue({ nome: "", cognome: "" });
    } else {
      this.form.patchValue({ denominazione: "" });
    }
  }

  async onSubmit(): Promise<void> {
    if (this.form.invalid) {
      this.form.markAllAsTouched();
      return;
    }

    const value = this.form.getRawValue();
    const request = {
      denominazione: this.subjectType() === "azienda" ? value.denominazione || null : null,
      nome: this.subjectType() === "persona" ? value.nome || null : null,
      cognome: this.subjectType() === "persona" ? value.cognome || null : null,
      idPaese: value.idPaese!,
      idCodice: value.idCodice!,
      codiceFiscale: value.codiceFiscale || null,
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
      codiceDestinatario: value.codiceDestinatario || null,
    };

    try {
      if (this.isEditMode()) {
        await this.clientService.updateClient(this.clientId()!, request);
        this.notify.success("Cliente aggiornato");
      } else {
        await this.clientService.createClient(request);
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
