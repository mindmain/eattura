import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  computed,
  effect,
  inject,
  input,
  OnInit,
  signal,
} from "@angular/core";
import { takeUntilDestroyed, toSignal } from "@angular/core/rxjs-interop";
import { FormArray, FormBuilder, FormGroup, ReactiveFormsModule, Validators } from "@angular/forms";
import { Router, RouterLink } from "@angular/router";
import { InvoiceStore } from "../../../core/stores/invoice.store";
import { ClientStore } from "../../../core/stores/client.store";
import { SettingsStore } from "../../../core/stores/settings.store";
import { NotificationService } from "../../../core/services/notification.service";
import {
  TIPO_DOCUMENTO,
  MODALITA_PAGAMENTO,
  CONDIZIONI_PAGAMENTO,
  NATURA,
  TIPO_CASSA,
  TIPO_RITENUTA,
  CAUSALE_PAGAMENTO_RITENUTA,
} from "../../../core/models/sdi.model";
import type {
  DatiCassaDetail,
  DatiRitenutaDetail,
  DatiBolloDetail,
  InvoiceDetail,
} from "../../../core/models/invoice.model";
import { TipoDocumentoPipe } from "../../../shared/pipes/tipo-documento.pipe";
import { ModalitaPagamentoPipe } from "../../../shared/pipes/modalita-pagamento.pipe";
import { CondizioniPagamentoPipe } from "../../../shared/pipes/condizioni-pagamento.pipe";
import { AppCurrencyPipe } from "../../../shared/pipes/app-currency.pipe";
import { ClientNamePipe } from "../../../shared/pipes/client-name.pipe";
import type { ClientSummary } from "../../../core/models/client.model";
import type { CreateInvoiceRequest, CreateLineRequest, DatiDdtDetail, CreateAttachmentRequest } from "../../../core/models/invoice.model";

interface LineItemFormValue {
  descrizione: string;
  quantita: number | null;
  unitaMisura: string;
  prezzoUnitario: number;
  aliquotaIva: number;
  natura: string;
}

@Component({
  selector: "app-invoice-form",
  standalone: true,
  imports: [
    ReactiveFormsModule,
    TipoDocumentoPipe,
    ModalitaPagamentoPipe,
    CondizioniPagamentoPipe,
    AppCurrencyPipe,
    ClientNamePipe,
    RouterLink,
  ],
  templateUrl: "./invoice-form.component.html",
  styleUrl: "./invoice-form.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class InvoiceFormComponent implements OnInit {
  private fb = inject(FormBuilder);
  readonly invoiceStore = inject(InvoiceStore);
  readonly clientStore = inject(ClientStore);
  readonly settingsStore = inject(SettingsStore);
  private router = inject(Router);
  private notify = inject(NotificationService);
  private destroyRef = inject(DestroyRef);

  /** Router input binding via withComponentInputBinding */
  id = input<string | undefined>();

  currentStep = signal(1);
  invoiceId = signal<string | null>(null);
  private loadedInvoice = signal<InvoiceDetail | null>(null);
  isEditMode = computed(() => !(!this.id() && !this.invoiceId()));

  // Expose clients signal to the template dropdowns
  readonly clients = this.clientStore.clients;

  // Inline SDI validation state (review step).
  validating = signal(false);
  validated = signal(false);
  validationErrors = signal<{ code: string; message: string }[]>([]);
  datiDdt = signal<DatiDdtDetail[]>([]);
  allegati = signal<CreateAttachmentRequest[]>([]);

  // SDI enum entries for dropdowns
  tipiDocumento = Object.entries(TIPO_DOCUMENTO);
  modalitaPagamento = Object.entries(MODALITA_PAGAMENTO);
  condizioniPagamento = Object.entries(CONDIZIONI_PAGAMENTO);
  naturaCodes = Object.entries(NATURA);
  tipiCassa = Object.entries(TIPO_CASSA);
  tipiRitenuta = Object.entries(TIPO_RITENUTA);
  causaliPagamentoRitenuta = Object.entries(CAUSALE_PAGAMENTO_RITENUTA);

  form = this.fb.group({
    // Step 1 - General data
    tipoDocumento: ["TD01", Validators.required],
    numero: ["", Validators.required],
    data: [new Date().toISOString().substring(0, 10), Validators.required],
    divisa: ["EUR", Validators.required],
    cedenteId: ["", Validators.required],
    cessionarioId: ["", Validators.required],
    causale: [""],
    collegataNumero: [""],
    collegataData: [""],

    // PA, Split Payment e Tracciabilità Appalti PA
    isPa: [false],
    isSplitPayment: [false],
    tipoDocumentoTracciabilita: ["ordine"],
    idDocumentoTracciabilita: [""],
    dataDocumentoTracciabilita: [""],
    codiceCig: ["", [Validators.pattern(/^[A-Za-z0-9]{10}$/)]],
    codiceCup: ["", [Validators.pattern(/^[A-Za-z0-9]{15}$/)]],

    // Step 2 - Line items
    linee: this.fb.array<FormGroup>([]),

    // Step 3 - Previdenza, Ritenute e Bollo
    isForfettario: [false],
    riaddebitaBollo: [false],

    // Cassa Previdenziale
    applicaCassa: [false],
    tipoCassa: ["TC04"],
    alCassa: [4.0],
    cassaSoggettaIva: [true],
    cassaSoggettaRitenuta: [false],

    // Ritenuta d'acconto
    applicaRitenuta: [false],
    tipoRitenuta: ["RT01"],
    aliquotaRitenuta: [20.0],
    causalePagamentoRitenuta: ["A"],

    // Bollo Virtuale
    applicaBollo: [false],
    importoBollo: [2.0],

    // Step 4 - Payment
    condizioniPagamento: ["TP02"],
    modalitaPagamento: ["MP05", Validators.required],
    importoPagamento: [0, [Validators.required, Validators.min(0)]],
    dataScadenzaPagamento: [""],
    iban: [""],
  });

  get linee(): FormArray {
    return this.form.get("linee") as FormArray;
  }

  /** Reactive form state bridge to Signals */
  private formState = toSignal(this.form.valueChanges, {
    initialValue: this.form.getRawValue(),
  });

  /** Computed running total of line items (taxable compensation) */
  readonly runningTotal = computed(() => {
    const state = this.formState();
    if (!state.linee) return 0;
    return (state.linee as LineItemFormValue[]).reduce((acc, line) => {
      const qty = Number(line?.quantita) || 1;
      const price = Number(line?.prezzoUnitario) || 0;
      return acc + qty * price;
    }, 0);
  });

  /** Computed real-time breakdown of compensi, casse, iva, bollo, ritenute, and netto a pagare */
  readonly financialSummary = computed(() => {
    const state = this.formState();
    const rawLinee = (state.linee ?? []) as LineItemFormValue[];

    let imponibileLinee = 0;
    let nonTaxableTotal = 0;
    const ivaMap = new Map<number, number>();

    for (const line of rawLinee) {
      const qty = Number(line?.quantita) || 1;
      const price = Number(line?.prezzoUnitario) || 0;
      const tot = qty * price;
      imponibileLinee += tot;

      const al = Number(line?.aliquotaIva) || 0;
      if (al === 0) {
        nonTaxableTotal += tot;
      } else {
        ivaMap.set(al, (ivaMap.get(al) || 0) + tot);
      }
    }

    // Cassa previdenziale
    let contributoCassa = 0;
    let cassaIva = 0;
    if (state.applicaCassa) {
      const alCassa = Number(state.alCassa) || 0;
      contributoCassa = Math.round(imponibileLinee * (alCassa / 100) * 100) / 100;
      if (state.cassaSoggettaIva && !state.isForfettario) {
        cassaIva = Math.round(contributoCassa * 0.22 * 100) / 100;
      }
    }

    // Totale IVA
    let totIva = 0;
    if (!state.isForfettario) {
      for (const [al, imp] of ivaMap.entries()) {
        totIva += Math.round(imp * (al / 100) * 100) / 100;
      }
      totIva += cassaIva;
      totIva = Math.round(totIva * 100) / 100;
    }

    // Bollo Virtuale (€2.00)
    const isExemptOver77 = nonTaxableTotal > 77.47 || (state.isForfettario && imponibileLinee > 77.47);
    let bolloAmt = 0;
    if (state.applicaBollo || isExemptOver77) {
      bolloAmt = Number(state.importoBollo) || 2.0;
    }

    // Totale Documento (Lordo)
    const bolloInLordo = state.riaddebitaBollo || state.applicaBollo || isExemptOver77;
    const totaleLordo = Math.round((imponibileLinee + contributoCassa + totIva + (bolloInLordo ? bolloAmt : 0)) * 100) / 100;

    // Ritenuta d'acconto
    let importoRitenuta = 0;
    if (state.applicaRitenuta && !state.isForfettario) {
      let baseRitenuta = imponibileLinee;
      if (state.cassaSoggettaRitenuta) {
        baseRitenuta += contributoCassa;
      }
      const alRit = Number(state.aliquotaRitenuta) || 20.0;
      importoRitenuta = Math.round(baseRitenuta * (alRit / 100) * 100) / 100;
    }

    // Split Payment deduction (IVA trattenuta e versata direttamente all'Erario dalla PA)
    const splitPaymentDeduction = state.isSplitPayment ? totIva : 0;

    // Netto a pagare
    const nettoAPagare = Math.max(0, Math.round((totaleLordo - importoRitenuta - splitPaymentDeduction) * 100) / 100);

    return {
      imponibileLinee,
      nonTaxableTotal,
      isExemptOver77,
      contributoCassa,
      cassaIva,
      totIva,
      bolloAmt,
      totaleLordo,
      importoRitenuta,
      splitPaymentDeduction,
      nettoAPagare,
    };
  });

  isAutofattura = computed(() => {
    const td = this.form.get("tipoDocumento")?.value;
    return ["TD16", "TD17", "TD18", "TD19", "TD20", "TD21", "TD27"].includes(td ?? "");
  });

  stepLabels = [
    "Dati Generali",
    "Linee Dettaglio",
    "Previdenza & Ritenute",
    "Pagamento",
    "Riepilogo",
  ];

  constructor() {
    effect(async () => {
      const paramId = this.id();
      if (!paramId) return;

      this.invoiceId.set(paramId);
      try {
        const inv = await this.invoiceStore.getInvoice(paramId);
        this.loadedInvoice.set(inv);
        this.form.patchValue({
          tipoDocumento: inv.tipoDocumento,
          numero: inv.numero,
          data: inv.data,
          divisa: inv.divisa,
          cedenteId: inv.cedenteId,
          cessionarioId: inv.cessionarioId,
          causale: inv.causale?.join("\n") ?? "",
          collegataNumero: inv.datiFattureCollegate?.[0]?.idDocumento ?? "",
          collegataData: inv.datiFattureCollegate?.[0]?.data ?? "",
        });
        // Populate line items (extract synthetic bollo recharge line if present alongside other lines)
        this.linee.clear();
        const hasSyntheticBolloLine =
          inv.linee.length > 1 &&
          inv.linee.some(
            (l) =>
              l.descrizione === "Riaddebito imposta di bollo" &&
              l.aliquotaIva === 0 &&
              (l.natura === "N1" || l.natura === "N2.2"),
          );
        const editableLines = hasSyntheticBolloLine
          ? inv.linee.filter(
              (l) =>
                !(
                  l.descrizione === "Riaddebito imposta di bollo" &&
                  l.aliquotaIva === 0 &&
                  (l.natura === "N1" || l.natura === "N2.2")
                ),
            )
          : inv.linee;
        if (hasSyntheticBolloLine) {
          this.form.patchValue({ riaddebitaBollo: true, applicaBollo: true });
        }
        for (const line of editableLines) {
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
        if (inv.isPa || inv.formatoTrasmissione === "FPA12") {
          this.form.patchValue({ isPa: true });
        }
        if (inv.isSplitPayment || inv.esigibilitaIva === "S") {
          this.form.patchValue({ isSplitPayment: true });
        }
        const trac = inv.datiOrdineAcquisto?.[0] || inv.datiContratto?.[0] || inv.datiConvenzione?.[0];
        if (trac) {
          const tipoTrac = inv.datiContratto?.length ? "contratto" : (inv.datiConvenzione?.length ? "convenzione" : "ordine");
          this.form.patchValue({
            tipoDocumentoTracciabilita: tipoTrac,
            idDocumentoTracciabilita: trac.idDocumento,
            dataDocumentoTracciabilita: trac.data ?? "",
            codiceCig: trac.codiceCig ?? "",
            codiceCup: trac.codiceCup ?? "",
          });
        }

        // Populate cassa previdenziale if available
        if (inv.datiCassaPrevidenziale && inv.datiCassaPrevidenziale.length > 0) {
          const cassa = inv.datiCassaPrevidenziale[0];
          this.form.patchValue({
            applicaCassa: true,
            tipoCassa: cassa.tipoCassa,
            alCassa: cassa.alCassa,
            cassaSoggettaIva: cassa.aliquotaIva > 0,
            cassaSoggettaRitenuta: cassa.ritenuta === "SI",
          });
        }

        // Populate ritenuta d'acconto if available
        if (inv.datiRitenuta && inv.datiRitenuta.length > 0) {
          const rit = inv.datiRitenuta[0];
          this.form.patchValue({
            applicaRitenuta: true,
            tipoRitenuta: rit.tipoRitenuta,
            aliquotaRitenuta: rit.aliquotaRitenuta,
            causalePagamentoRitenuta: rit.causalePagamento,
          });
        }

        // Populate bollo virtuale if available
        if (inv.datiBollo && inv.datiBollo.bolloVirtuale === "SI") {
          this.form.patchValue({
            applicaBollo: true,
            importoBollo: inv.datiBollo.importoBollo ?? 2.0,
          });
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

        if (inv.datiDdt && inv.datiDdt.length > 0) {
          this.datiDdt.set(inv.datiDdt);
        }
        if (inv.allegati && inv.allegati.length > 0) {
          this.allegati.set(inv.allegati.map((a) => ({
            nomeAttachment: a.nomeAttachment,
            algoritmoCompressione: a.algoritmoCompressione,
            formatoAttachment: a.formatoAttachment,
            descrizioneAttachment: a.descrizioneAttachment,
            attachmentBase64: a.attachmentBase64,
          })));
        }
      } catch {
        this.notify.error("Errore nel caricamento della fattura");
        this.router.navigate(["/invoices"]);
      }
    });
  }

  async ngOnInit(): Promise<void> {
    try {
      await Promise.all([
        this.clientStore.loadAll(),
        this.settingsStore.load(),
      ]);
    } catch {
      this.notify.error("Errore nel caricamento delle anagrafiche");
    }

    if (!this.id()) {
      const navDraft = history.state?.autofatturaDraft as CreateInvoiceRequest | undefined;
      if (navDraft) {
        this.loadAutofatturaDraft(navDraft);
      } else {
        // Start with one empty line
        this.addLine();
      }

      // Automatically preselect the user's company as Cedente if configured
      const defaultCedente = this.settingsStore.findCompanyClientId(this.clientStore.clients());
      if (defaultCedente) {
        this.form.patchValue({ cedenteId: defaultCedente });
        void this.suggestNumber(defaultCedente);
      }

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
      return;
    }
    const data = this.form.get("data")!.value;
    const year = data && data.length >= 4 ? data.substring(0, 4) : String(new Date().getFullYear());
    try {
      const next = await this.invoiceStore.nextInvoiceNumber(cedenteId, year);
      if (!this.form.get("numero")!.value) {
        this.form.patchValue({ numero: next });
      }
    } catch {
      // Non-critical: leave the field empty for manual entry.
    }
  }

  addDdt(): void {
    this.datiDdt.update((list) => [
      ...list,
      {
        numeroDdt: "",
        dataDdt: new Date().toISOString().substring(0, 10),
        riferimentoNumeroLinea: [],
      },
    ]);
  }

  removeDdt(index: number): void {
    this.datiDdt.update((list) => list.filter((_, i) => i !== index));
  }

  updateDdtNumber(index: number, val: string): void {
    this.datiDdt.update((list) => {
      const copy = [...list];
      copy[index] = { ...copy[index], numeroDdt: val };
      return copy;
    });
  }

  updateDdtDate(index: number, val: string): void {
    this.datiDdt.update((list) => {
      const copy = [...list];
      copy[index] = { ...copy[index], dataDdt: val };
      return copy;
    });
  }

  updateDdtLinee(index: number, val: string): void {
    const lines = val
      .split(",")
      .map((s) => parseInt(s.trim(), 10))
      .filter((n) => !isNaN(n));
    this.datiDdt.update((list) => {
      const copy = [...list];
      copy[index] = { ...copy[index], riferimentoNumeroLinea: lines };
      return copy;
    });
  }

  onFileUpload(event: Event): void {
    const input = event.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;

    for (let i = 0; i < input.files.length; i++) {
      const file = input.files[i];
      if (file.size > 5 * 1024 * 1024) {
        this.notify.warning(`Attenzione: "${file.name}" supera i 5 MB raccomandati da SDI.`);
      }
      const reader = new FileReader();
      reader.onload = () => {
        const result = reader.result as string;
        const base64 = result.includes(",") ? result.split(",")[1] : result;
        const ext = file.name.split(".").pop()?.toUpperCase() || "FILE";
        this.allegati.update((list) => [
          ...list,
          {
            nomeAttachment: file.name,
            formatoAttachment: ext,
            descrizioneAttachment: file.name,
            attachmentBase64: base64,
          },
        ]);
        this.notify.success(`Allegato "${file.name}" caricato correttamente`);
      };
      reader.readAsDataURL(file);
    }
    input.value = "";
  }

  removeAllegato(index: number): void {
    this.allegati.update((list) => list.filter((_, i) => i !== index));
  }

  updateAllegatoDesc(index: number, desc: string): void {
    this.allegati.update((list) => {
      const copy = [...list];
      copy[index] = { ...copy[index], descrizioneAttachment: desc };
      return copy;
    });
  }

  onCessionarioChange(): void {
    const buyerId = this.form.get("cessionarioId")?.value;
    const buyer = this.clients().find((cl) => cl.id === buyerId);
    if (buyer) {
      const isPa = !!(buyer.isPa || (buyer.codiceDestinatario && buyer.codiceDestinatario.length === 6));
      if (isPa) {
        this.form.patchValue({
          isPa: true,
          isSplitPayment: true,
        });
      }
    }
  }

  nextStep(): void {
    if (this.currentStep() < 5) {
      this.currentStep.update((s) => s + 1);
    }
  }

  prevStep(): void {
    if (this.currentStep() > 1) {
      this.currentStep.update((s) => s - 1);
    }
  }

  loadAutofatturaDraft(draft: CreateInvoiceRequest): void {
    const collegata = draft.datiFattureCollegate?.[0];
    this.form.patchValue({
      tipoDocumento: draft.tipoDocumento,
      numero: draft.numero,
      data: draft.data,
      divisa: draft.divisa,
      cedenteId: draft.cedenteId,
      cessionarioId: draft.cessionarioId,
      causale: draft.causale?.join("\n") ?? "",
      collegataNumero: collegata?.idDocumento ?? "",
      collegataData: collegata?.data ?? "",
      modalitaPagamento: draft.pagamenti?.[0]?.modalitaPagamento ?? "MP05",
      importoPagamento: draft.pagamenti?.[0]?.importoPagamento ?? 0,
    });

    this.linee.clear();
    for (const l of draft.linee) {
      this.linee.push(
        this.fb.group({
          descrizione: [l.descrizione, Validators.required],
          quantita: [l.quantita ?? 1],
          unitaMisura: [l.unitaMisura ?? ""],
          prezzoUnitario: [l.prezzoUnitario, Validators.required],
          aliquotaIva: [l.aliquotaIva, Validators.required],
          natura: [l.natura ?? ""],
        })
      );
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
    return this.runningTotal();
  }

  syncPaymentAmount(): void {
    const summary = this.financialSummary();
    this.form.patchValue({ importoPagamento: summary.nettoAPagare });
  }

  toggleForfettario(enabled: boolean): void {
    this.form.patchValue({ isForfettario: enabled });
    if (enabled) {
      // In regime forfettario:
      // 1. All lines have 0% VAT with Natura N2.2
      for (let i = 0; i < this.linee.length; i++) {
        const line = this.linee.at(i);
        line.patchValue({
          aliquotaIva: 0,
          natura: "N2.2",
        });
      }
      // 2. No withholding tax allowed
      this.form.patchValue({
        applicaRitenuta: false,
        cassaSoggettaIva: false,
      });

      // 3. Automatically append statutory mandatory note to Causale if not present
      const statutoryNote = "Operazione effettuata in regime forfettario ai sensi dell'art. 1, commi da 54 a 89, della Legge n. 190/2014 e succ. mod. Compensi non soggetti ad IVA ai sensi dell'art. 1, comma 58, L. 190/2014 e non soggetti a ritenuta d'acconto ai sensi dell'art. 1, comma 67, L. 190/2014.";
      const currentCausale = this.form.get("causale")?.value || "";
      if (!currentCausale.includes("Legge n. 190/2014")) {
        const updated = currentCausale.trim() ? `${currentCausale.trim()}\n${statutoryNote}` : statutoryNote;
        this.form.patchValue({ causale: updated });
      }

      // 4. Auto-enable bollo virtuale if lines sum > 77.47
      if (this.runningTotal() > 77.47) {
        this.form.patchValue({ applicaBollo: true });
      }
    }
  }

  clientDisplayName(client: ClientSummary): string {
    return client.denominazione ?? `${client.nome ?? ""} ${client.cognome ?? ""}`.trim();
  }

  clientNameById(id: string): string {
    const c = this.clientStore.clientMap().get(id);
    return c ? this.clientDisplayName(c) : id;
  }

  private buildRequest(): CreateInvoiceRequest {
    const val = this.form.getRawValue();
    const summary = this.financialSummary();
    const orig = this.loadedInvoice();
    const rawLinee = (val.linee ?? []) as LineItemFormValue[];
    const linee: CreateLineRequest[] = rawLinee.map((l) => ({
      descrizione: l.descrizione,
      quantita: l.quantita || null,
      unitaMisura: l.unitaMisura || null,
      prezzoUnitario: Number(l.prezzoUnitario),
      aliquotaIva: Number(l.aliquotaIva),
      natura: l.natura || null,
    }));

    // If client requested bollo recharge as invoice line item, avoid duplicating if already present
    if (
      val.riaddebitaBollo &&
      summary.bolloAmt > 0 &&
      !linee.some((l) => l.descrizione === "Riaddebito imposta di bollo")
    ) {
      linee.push({
        descrizione: "Riaddebito imposta di bollo",
        quantita: 1,
        unitaMisura: null,
        prezzoUnitario: summary.bolloAmt,
        aliquotaIva: 0,
        natura: val.isForfettario ? "N2.2" : "N1",
      });
    }

    // Build Casse Previdenziali (preserving any additional entries from loaded invoice)
    let datiCassaPrevidenziale: DatiCassaDetail[] | null = null;
    if (val.applicaCassa) {
      const extraCasse = orig?.datiCassaPrevidenziale?.slice(1) ?? [];
      datiCassaPrevidenziale = [
        {
          tipoCassa: val.tipoCassa || "TC04",
          alCassa: Number(val.alCassa) || 4.0,
          importoContributoCassa: summary.contributoCassa,
          imponibileCassa: summary.imponibileLinee,
          aliquotaIva: val.isForfettario || !val.cassaSoggettaIva ? 0.0 : 22.0,
          ritenuta: val.cassaSoggettaRitenuta ? "SI" : null,
          natura: val.isForfettario ? "N2.2" : null,
        },
        ...extraCasse,
      ];
    }

    // Build Ritenute d'Acconto (preserving any additional entries from loaded invoice)
    let datiRitenuta: DatiRitenutaDetail[] | null = null;
    if (val.applicaRitenuta && !val.isForfettario && summary.importoRitenuta > 0) {
      const extraRitenute = orig?.datiRitenuta?.slice(1) ?? [];
      datiRitenuta = [
        {
          tipoRitenuta: val.tipoRitenuta || "RT01",
          importoRitenuta: summary.importoRitenuta,
          aliquotaRitenuta: Number(val.aliquotaRitenuta) || 20.0,
          causalePagamento: val.causalePagamentoRitenuta || "A",
        },
        ...extraRitenute,
      ];
    }

    // Build Bollo Virtuale
    let datiBollo: DatiBolloDetail | null = null;
    if (val.applicaBollo || summary.isExemptOver77) {
      datiBollo = {
        bolloVirtuale: "SI",
        importoBollo: summary.bolloAmt,
      };
    }

    // Parse causale lines
    const causaleLines = val.causale
      ? val.causale.split("\n").map((s: string) => s.trim()).filter((s: string) => s.length > 0)
      : [];

    const extraPagamenti = (orig?.pagamenti?.slice(1) ?? []).map((p) => ({
      modalitaPagamento: p.modalitaPagamento,
      importoPagamento: p.importoPagamento,
      dataScadenzaPagamento: p.dataScadenzaPagamento,
      iban: p.iban,
    }));

    const extraCollegate = orig?.datiFattureCollegate?.slice(1) ?? [];

    return {
      numero: val.numero!,
      data: val.data!,
      tipoDocumento: val.tipoDocumento!,
      divisa: val.divisa!,
      cedenteId: val.cedenteId!,
      cessionarioId: val.cessionarioId!,
      causale: causaleLines,
      linee,
      pagamenti: [
        {
          modalitaPagamento: val.modalitaPagamento!,
          importoPagamento: Number(val.importoPagamento),
          dataScadenzaPagamento: val.dataScadenzaPagamento || null,
          iban: val.iban || null,
        },
        ...extraPagamenti,
      ],
      datiCassaPrevidenziale,
      datiRitenuta,
      datiBollo,
      isAutofattura: this.isAutofattura(),
      soggettoEmittente: this.isAutofattura() ? "CC" : null,
      datiFattureCollegate: val.collegataNumero
        ? [
            { idDocumento: val.collegataNumero, data: val.collegataData || null },
            ...extraCollegate,
          ]
        : extraCollegate.length > 0
          ? extraCollegate
          : null,
      isPa: !!val.isPa,
      isSplitPayment: !!val.isSplitPayment,
      esigibilitaIva: val.isSplitPayment ? "S" : null,
      datiOrdineAcquisto: (val.idDocumentoTracciabilita || val.codiceCig || val.codiceCup) && val.tipoDocumentoTracciabilita === "ordine"
        ? [{
            tipoDocumentoRif: "ordine",
            idDocumento: val.idDocumentoTracciabilita || (val.codiceCig ? `CIG-${val.codiceCig}` : "ORD-1"),
            data: val.dataDocumentoTracciabilita || null,
            codiceCig: val.codiceCig ? val.codiceCig.trim() : null,
            codiceCup: val.codiceCup ? val.codiceCup.trim() : null,
          }, ...(orig?.datiOrdineAcquisto?.slice(1) ?? [])]
        : null,
      datiContratto: (val.idDocumentoTracciabilita || val.codiceCig || val.codiceCup) && val.tipoDocumentoTracciabilita === "contratto"
        ? [{
            tipoDocumentoRif: "contratto",
            idDocumento: val.idDocumentoTracciabilita || (val.codiceCig ? `CIG-${val.codiceCig}` : "CONTR-1"),
            data: val.dataDocumentoTracciabilita || null,
            codiceCig: val.codiceCig ? val.codiceCig.trim() : null,
            codiceCup: val.codiceCup ? val.codiceCup.trim() : null,
          }, ...(orig?.datiContratto?.slice(1) ?? [])]
        : null,
      datiConvenzione: (val.idDocumentoTracciabilita || val.codiceCig || val.codiceCup) && val.tipoDocumentoTracciabilita === "convenzione"
        ? [{
            tipoDocumentoRif: "convenzione",
            idDocumento: val.idDocumentoTracciabilita || (val.codiceCig ? `CIG-${val.codiceCig}` : "CONV-1"),
            data: val.dataDocumentoTracciabilita || null,
            codiceCig: val.codiceCig ? val.codiceCig.trim() : null,
            codiceCup: val.codiceCup ? val.codiceCup.trim() : null,
          }, ...(orig?.datiConvenzione?.slice(1) ?? [])]
        : null,
      datiDdt: this.datiDdt().length > 0 ? this.datiDdt() : null,
      allegati: this.allegati().length > 0 ? this.allegati() : null,
    };
  }

  private async saveDraft(): Promise<string> {
    const request = this.buildRequest();
    const editId = this.invoiceId();
    if (this.isEditMode() && editId) {
      const updated = await this.invoiceStore.updateInvoice(editId, request);
      const nextId = updated?.id || editId;
      this.invoiceId.set(nextId);
      return nextId;
    }
    const created = await this.invoiceStore.createInvoice(request);
    this.invoiceId.set(created.id);
    return created.id;
  }

  async validateDraft(): Promise<void> {
    if (this.form.invalid) {
      this.form.markAllAsTouched();
      this.notify.error("Compila i campi obbligatori prima di validare");
      return;
    }
    this.validating.set(true);
    try {
      const id = await this.saveDraft();
      const result = await this.invoiceStore.validateInvoice(id);
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
    if (this.form.invalid) {
      this.form.markAllAsTouched();
      this.notify.error("Compila i campi obbligatori prima di salvare");
      return;
    }

    const request = this.buildRequest();

    try {
      const editId = this.invoiceId();
      if (this.isEditMode() && editId) {
        await this.invoiceStore.updateInvoice(editId, request);
        this.notify.success("Fattura aggiornata");
      } else {
        await this.invoiceStore.createInvoice(request);
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
