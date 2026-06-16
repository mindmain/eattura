import { Component, DestroyRef, computed, inject, OnInit, signal } from "@angular/core";
import { takeUntilDestroyed } from "@angular/core/rxjs-interop";
import { FormArray, FormBuilder, FormGroup, ReactiveFormsModule, Validators } from "@angular/forms";
import { ActivatedRoute, Router } from "@angular/router";
import { InvoiceService } from "../../../core/services/invoice.service";
import { ClientService } from "../../../core/services/client.service";
import { NotificationService } from "../../../core/services/notification.service";
import {
  TIPO_DOCUMENTO,
  MODALITA_PAGAMENTO,
  CONDIZIONI_PAGAMENTO,
  NATURA,
} from "../../../core/models/sdi.model";
import type { ClientSummary } from "../../../core/models/client.model";
import type { CreateInvoiceRequest } from "../../../core/models/invoice.model";

@Component({
  selector: "app-invoice-form",
  standalone: true,
  imports: [ReactiveFormsModule],
  templateUrl: "./invoice-form.component.html",
  styleUrl: "./invoice-form.component.css",
})
export class InvoiceFormComponent implements OnInit {
  private fb = inject(FormBuilder);
  private invoiceService = inject(InvoiceService);
  private clientService = inject(ClientService);
  private router = inject(Router);
  private route = inject(ActivatedRoute);
  private notify = inject(NotificationService);
  private destroyRef = inject(DestroyRef);

  currentStep = signal(1);
  isEditMode = signal(false);
  invoiceId = signal<string | null>(null);
  clients = signal<ClientSummary[]>([]);

  // Inline SDI validation state (review step).
  validating = signal(false);
  validated = signal(false);
  validationErrors = signal<{ code: string; message: string }[]>([]);

  // SDI enum entries for dropdowns
  tipiDocumento = Object.entries(TIPO_DOCUMENTO);
  modalitaPagamento = Object.entries(MODALITA_PAGAMENTO);
  condizioniPagamento = Object.entries(CONDIZIONI_PAGAMENTO);
  naturaCodes = Object.entries(NATURA);

  form = this.fb.group({
    // Step 1 - General data
    tipoDocumento: ["TD01", Validators.required],
    numero: ["", Validators.required],
    data: ["", Validators.required],
    divisa: ["EUR", Validators.required],
    cedenteId: ["", Validators.required],
    cessionarioId: ["", Validators.required],
    causale: [""],
    // Step 2 - Line items
    linee: this.fb.array([]),
    // Step 3 - Payment
    condizioniPagamento: ["TP02"],
    modalitaPagamento: ["MP05", Validators.required],
    importoPagamento: [0, [Validators.required, Validators.min(0)]],
    dataScadenzaPagamento: [""],
    iban: [""],
  });

  get linee(): FormArray {
    return this.form.get("linee") as FormArray;
  }

  /** Computed running total from line items. */
  runningTotal = computed(() => {
    // Recompute by reading signal to trigger reactivity
    let total = 0;
    for (let i = 0; i < this.linee.length; i++) {
      const line = this.linee.at(i) as FormGroup;
      const qty = Number(line.get("quantita")?.value) || 1;
      const price = Number(line.get("prezzoUnitario")?.value) || 0;
      total += qty * price;
    }
    return total;
  });

  stepLabels = ["Dati Generali", "Linee Dettaglio", "Pagamento", "Riepilogo"];

  async ngOnInit(): Promise<void> {
    // Load clients for dropdown
    try {
      const list = await this.clientService.listClients();
      this.clients.set(list);
    } catch {
      this.notify.error("Errore nel caricamento dei clienti");
    }

    // Check edit mode
    const id = this.route.snapshot.paramMap.get("id");
    if (id) {
      this.isEditMode.set(true);
      this.invoiceId.set(id);
      try {
        const inv = await this.invoiceService.getInvoice(id);
        this.form.patchValue({
          tipoDocumento: inv.tipoDocumento,
          numero: inv.numero,
          data: inv.data,
          divisa: inv.divisa,
          cedenteId: inv.cedenteId,
          cessionarioId: inv.cessionarioId,
          causale: inv.causale?.join("\n") ?? "",
        });
        // Populate line items
        for (const line of inv.linee) {
          this.linee.push(
            this.fb.group({
              descrizione: [line.descrizione, Validators.required],
              quantita: [line.quantita ?? 1],
              unitaMisura: [line.unitaMisura ?? ""],
              prezzoUnitario: [line.prezzoUnitario, Validators.required],
              aliquotaIva: [line.aliquotaIva, Validators.required],
              natura: [line.natura ?? ""],
            })
          );
        }
        // Populate payment if available
        if (inv.pagamenti?.length > 0) {
          const p = inv.pagamenti[0];
          this.form.patchValue({
            modalitaPagamento: p.modalitaPagamento,
            importoPagamento: p.importoPagamento,
            dataScadenzaPagamento: p.dataScadenzaPagamento ?? "",
            iban: p.iban ?? "",
          });
        }
      } catch {
        this.notify.error("Errore nel caricamento della fattura");
        this.router.navigate(["/invoices"]);
      }
    } else {
      // Start with one empty line
      this.addLine();
      // Suggest a progressive invoice number once a cedente is selected.
      this.form
        .get("cedenteId")!
        .valueChanges.pipe(takeUntilDestroyed(this.destroyRef))
        .subscribe((cedenteId) => {
          void this.suggestNumber(cedenteId);
        });
    }
  }

  /** Pre-fill `numero` with the next progressive value for the chosen cedente/year. */
  private async suggestNumber(cedenteId: string | null): Promise<void> {
    if (!cedenteId || this.isEditMode() || this.form.get("numero")!.value) {
      return; // nothing selected, editing, or the user already typed a number
    }
    const data = this.form.get("data")!.value;
    const year = data && data.length >= 4 ? data.substring(0, 4) : String(new Date().getFullYear());
    try {
      const next = await this.invoiceService.nextInvoiceNumber(cedenteId, year);
      if (!this.form.get("numero")!.value) {
        this.form.patchValue({ numero: next });
      }
    } catch {
      // Non-critical: leave the field empty for manual entry.
    }
  }

  nextStep(): void {
    if (this.currentStep() < 4) {
      this.currentStep.update((s) => s + 1);
    }
  }

  prevStep(): void {
    if (this.currentStep() > 1) {
      this.currentStep.update((s) => s - 1);
    }
  }

  addLine(): void {
    this.linee.push(
      this.fb.group({
        descrizione: ["", Validators.required],
        quantita: [1],
        unitaMisura: ["pz"],
        prezzoUnitario: [0, Validators.required],
        aliquotaIva: [22, Validators.required],
        natura: [""],
      })
    );
  }

  removeLine(index: number): void {
    this.linee.removeAt(index);
  }

  lineTotal(index: number): number {
    const line = this.linee.at(index) as FormGroup;
    const qty = Number(line.get("quantita")?.value) || 1;
    const price = Number(line.get("prezzoUnitario")?.value) || 0;
    return qty * price;
  }

  computeTotal(): number {
    let total = 0;
    for (let i = 0; i < this.linee.length; i++) {
      total += this.lineTotal(i);
    }
    return total;
  }

  syncPaymentAmount(): void {
    this.form.patchValue({ importoPagamento: this.computeTotal() });
  }

  clientDisplayName(client: ClientSummary): string {
    return client.denominazione ?? `${client.nome ?? ""} ${client.cognome ?? ""}`.trim();
  }

  tipoLabel(code: string): string {
    return TIPO_DOCUMENTO[code] ?? code;
  }

  modalitaLabel(code: string): string {
    return MODALITA_PAGAMENTO[code] ?? code;
  }

  condizioniLabel(code: string): string {
    return CONDIZIONI_PAGAMENTO[code] ?? code;
  }

  clientNameById(id: string): string {
    const c = this.clients().find((cl) => cl.id === id);
    return c ? this.clientDisplayName(c) : id;
  }

  formatCurrency(value: number): string {
    return new Intl.NumberFormat("it-IT", { style: "currency", currency: "EUR" }).format(value);
  }

  private buildRequest(): CreateInvoiceRequest {
    const val = this.form.getRawValue();
    return {
      numero: val.numero!,
      data: val.data!,
      tipoDocumento: val.tipoDocumento!,
      divisa: val.divisa!,
      cedenteId: val.cedenteId!,
      cessionarioId: val.cessionarioId!,
      linee: val.linee!.map((l: any) => ({
        descrizione: l.descrizione,
        quantita: l.quantita || null,
        unitaMisura: l.unitaMisura || null,
        prezzoUnitario: Number(l.prezzoUnitario),
        aliquotaIva: Number(l.aliquotaIva),
        natura: l.natura || null,
      })),
      pagamenti: [
        {
          modalitaPagamento: val.modalitaPagamento!,
          importoPagamento: Number(val.importoPagamento),
          dataScadenzaPagamento: val.dataScadenzaPagamento || null,
          iban: val.iban || null,
        },
      ],
    };
  }

  /**
   * Persist the current form as a draft (create or update) and return its ID.
   * The invoice stays in `draft` state, so this is safe to call repeatedly.
   */
  private async saveDraft(): Promise<string> {
    const request = this.buildRequest();
    const editId = this.invoiceId();
    if (this.isEditMode() && editId) {
      await this.invoiceService.updateInvoice(editId, request);
      return editId;
    }
    const created = await this.invoiceService.createInvoice(request);
    // Switch to edit mode so subsequent saves update the same draft.
    this.isEditMode.set(true);
    this.invoiceId.set(created.id);
    return created.id;
  }

  /** Save the draft and run SDI validation, showing any errors inline. */
  async validateDraft(): Promise<void> {
    if (this.form.invalid) {
      this.form.markAllAsTouched();
      this.notify.error("Compila i campi obbligatori prima di validare");
      return;
    }
    this.validating.set(true);
    try {
      const id = await this.saveDraft();
      const result = await this.invoiceService.validateInvoice(id);
      this.validationErrors.set(result.errors);
      this.validated.set(result.valid);
      if (result.valid) {
        this.notify.success("Fattura valida e conforme alle specifiche SDI");
      } else {
        this.notify.error(`Validazione fallita: ${result.errors.length} errori`);
      }
    } catch (e) {
      this.notify.error(
        typeof e === "string" ? e : "Errore durante la validazione"
      );
    } finally {
      this.validating.set(false);
    }
  }

  async onSubmit(): Promise<void> {
    const request = this.buildRequest();

    try {
      const editId = this.invoiceId();
      if (this.isEditMode() && editId) {
        await this.invoiceService.updateInvoice(editId, request);
        this.notify.success("Fattura aggiornata");
      } else {
        await this.invoiceService.createInvoice(request);
        this.notify.success("Fattura creata");
      }
      this.router.navigate(["/invoices"]);
    } catch (e) {
      this.notify.error(
        typeof e === "string" ? e : "Errore durante il salvataggio della fattura"
      );
    }
  }

  cancel(): void {
    this.router.navigate(["/invoices"]);
  }
}
