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
import type { ValidatorFn } from "@angular/forms";
import { Router, RouterLink } from "@angular/router";
import { InvoiceStore } from "../../../core/stores/invoice.store";
import { ClientStore } from "../../../core/stores/client.store";
import { SettingsStore } from "../../../core/stores/settings.store";
import { NotificationService } from "../../../core/services/notification.service";
import {
  ALIQUOTA_IVA_PRESETS,
  BOLLO_SUBJECT_NATURA_CODES,
  CAUSALE_PAGAMENTO_RITENUTA,
  CONDIZIONI_PAGAMENTO,
  CONDIZIONI_PAGAMENTO_DETAILS,
  MODALITA_PAGAMENTO,
  MODALITA_PAGAMENTO_DETAILS,
  NATURA,
  NATURA_DETAILS,
  SELECTABLE_NATURA_CODES,
  TIPO_CASSA,
  TIPO_DOCUMENTO,
  TIPO_RITENUTA,
} from "../../../core/models/sdi.model";
import type {
  CreateAttachmentRequest,
  CreateInvoiceRequest,
  CreateLineRequest,
  DatiBolloDetail,
  DatiCassaDetail,
  DatiDdtDetail,
  DatiRitenutaDetail,
  InvoiceDetail,
  InvoiceLineFields,
} from "../../../core/models/invoice.model";
import { TipoDocumentoPipe } from "../../../shared/pipes/tipo-documento.pipe";
import { ModalitaPagamentoPipe } from "../../../shared/pipes/modalita-pagamento.pipe";
import { CondizioniPagamentoPipe } from "../../../shared/pipes/condizioni-pagamento.pipe";
import { AppCurrencyPipe } from "../../../shared/pipes/app-currency.pipe";
import { ClientNamePipe } from "../../../shared/pipes/client-name.pipe";
import type { ClientSummary } from "../../../core/models/client.model";
import {
  DocSelectComponent,
  type DocSelectOption,
} from "../../../shared/components/doc-select/doc-select.component";
import { DatePickerComponent } from "../../../shared/components/date-picker/date-picker.component";

export interface LineItemFormValue {
  descrizione: string;
  quantita: number | null;
  unitaMisura: string;
  dataInizioPeriodo: string;
  dataFinePeriodo: string;
  prezzoUnitario: number;
  aliquotaIva: number;
  natura: string;
  fields: InvoiceLineFields;
}

export const STATUTORY_FORFETTARIO_CAUSALE_LINES: readonly string[] = [
  "Operazione effettuata in regime forfettario ai sensi dell'art. 1, commi da 54 a 89, della Legge n. 190/2014 e succ. mod.",
  "Compensi non soggetti ad IVA ai sensi dell'art. 1, comma 58, L. 190/2014 e non soggetti a ritenuta d'acconto ai sensi dell'art. 1, comma 67, L. 190/2014.",
];

const sdiDescriptionValidator: ValidatorFn = (control) => {
  const value: unknown = control.value;
  if (typeof value !== "string" || !value || /^[\t\n\r \u0085\u00a0]+$/u.test(value)) {
    return { required: true };
  }

  // Match the Rust check for XML 1.0 and the SdI String1000LatinType.
  return value.length > 1000 || /[^\t\n\r\u0020-\u00ff]/u.test(value)
    ? { sdiDescription: true }
    : null;
};

function lineAmount(line: Pick<LineItemFormValue, "quantita" | "prezzoUnitario" | "fields">): number {
  const unitPrice = (line.fields?.scontoMaggiorazione ?? []).reduce((price, item) => {
    const amount = item.importo ?? (price * (item.percentuale ?? 0)) / 100;
    return price + (item.tipo === "SC" ? -amount : amount);
  }, Number(line.prezzoUnitario) || 0);
  return Math.round(unitPrice * (line.quantita ?? 1) * 100) / 100;
}

/** Splits each causale paragraph into <=200-char chunks per FatturaPA XSD String200LatinType. */
function splitCausaleChunks(text: string): string[] {
  const out: string[] = [];
  for (const rawLine of text.split("\n")) {
    const trimmed = rawLine.trim();
    if (!trimmed) continue;
    if (trimmed.length <= 200) {
      out.push(trimmed);
      continue;
    }
    let current = "";
    for (const word of trimmed.split(/\s+/)) {
      if (current && current.length + 1 + word.length > 200) {
        out.push(current);
        current = "";
      }
      if (word.length > 200) {
        for (let i = 0; i < word.length; i += 200) {
          const piece = word.slice(i, i + 200);
          if (piece.length === 200) out.push(piece);
          else current = piece;
        }
      } else {
        current = current ? `${current} ${word}` : word;
      }
    }
    if (current) out.push(current);
  }
  return out;
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
    DocSelectComponent,
    DatePickerComponent,
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

  /** Tracks whether the user explicitly unchecked Bollo Virtuale despite legal threshold (> €77.47). */
  readonly bolloManuallyDisabled = signal(false);
  /** Index of the line currently loaded in the bottom editor (`null` when adding a new line). */
  readonly editingLineIndex = signal<number | null>(null);
  /** Tracks whether the user manually customized `importoPagamento` in Step 4. */
  private paymentAmountCustomized = signal(false);

  // Expose clients signal to the template dropdowns
  readonly clients = this.clientStore.clients;

  // Inline SDI validation state (review step).
  validating = signal(false);
  validated = signal(false);
  validationErrors = signal<{ code: string; message: string }[]>([]);
  datiDdt = signal<DatiDdtDetail[]>([]);
  allegati = signal<CreateAttachmentRequest[]>([]);

  // Documentary select options for DocSelectComponent
  readonly tipoDocumentoOptions: readonly DocSelectOption<string>[] = Object.entries(TIPO_DOCUMENTO).map(
    ([code, label]) => ({ value: code, code, label }),
  );
  readonly naturaOptions: readonly DocSelectOption<string>[] = SELECTABLE_NATURA_CODES.map((code) => ({
    value: code,
    code,
    label: NATURA[code] ?? code,
    detail: NATURA_DETAILS[code],
  }));
  readonly condizioniPagamentoOptions: readonly DocSelectOption<string>[] = Object.entries(
    CONDIZIONI_PAGAMENTO,
  ).map(([code, label]) => ({
    value: code,
    code,
    label,
    detail: CONDIZIONI_PAGAMENTO_DETAILS[code],
  }));
  readonly modalitaPagamentoOptions: readonly DocSelectOption<string>[] = Object.entries(
    MODALITA_PAGAMENTO,
  ).map(([code, label]) => ({
    value: code,
    code,
    label,
    detail: MODALITA_PAGAMENTO_DETAILS[code],
  }));
  readonly tipoCassaOptions: readonly DocSelectOption<string>[] = Object.entries(TIPO_CASSA).map(
    ([code, full]) => ({
      value: code,
      code,
      label: full.replace(`${code} - `, ""),
    }),
  );
  readonly tipoRitenutaOptions: readonly DocSelectOption<string>[] = Object.entries(TIPO_RITENUTA).map(
    ([code, full]) => ({
      value: code,
      code,
      label: full.replace(`${code} - `, ""),
    }),
  );
  readonly causaleRitenutaOptions: readonly DocSelectOption<string>[] = Object.entries(
    CAUSALE_PAGAMENTO_RITENUTA,
  ).map(([code, full]) => ({
    value: code,
    code,
    label: full.replace(`${code} - `, ""),
  }));

  private customIvaRate = signal<number | null>(null);
  readonly aliquotaIvaOptions = computed<readonly DocSelectOption<number>[]>(() => {
    const custom = this.customIvaRate();
    if (custom === null || ALIQUOTA_IVA_PRESETS.some((opt) => opt.value === custom)) {
      return ALIQUOTA_IVA_PRESETS;
    }
    return [
      ...ALIQUOTA_IVA_PRESETS,
      {
        value: custom,
        code: `${custom}%`,
        label: `Aliquota speciale (${custom}%)`,
        detail: "Aliquota IVA o percentuale di compensazione presente nel documento.",
      },
    ];
  });

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

    // Step 2 - Added line items
    linee: this.fb.array<FormGroup>([]),

    // Step 3 - Previdenza, Ritenute e Bollo
    isForfettario: [false],
    includiCausaleForfettario: [true],
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

  /** Dedicated bottom form for composing or editing a single line item in Step 2. */
  lineDraftForm = this.fb.group({
    descrizione: ["", sdiDescriptionValidator],
    quantita: [1 as number | null],
    unitaMisura: ["pz"],
    prezzoUnitario: [0, [Validators.required, Validators.min(0)]],
    aliquotaIva: [22, Validators.required],
    natura: [""],
    dataInizioPeriodo: [""],
    dataFinePeriodo: [""],
    scontoTipo: ["" as "" | "SC" | "MG"],
    scontoModo: ["percentuale" as "percentuale" | "importo"],
    scontoValore: [null as number | null],
    fields: [{} as InvoiceLineFields],
  });

  get linee(): FormArray {
    return this.form.get("linee") as FormArray;
  }

  /** Reactive form state bridge to Signals */
  private formState = toSignal(this.form.valueChanges, {
    initialValue: this.form.getRawValue(),
  });

  private lineDraftState = toSignal(this.lineDraftForm.valueChanges, {
    initialValue: this.lineDraftForm.getRawValue(),
  });

  /** Whether the currently selected Cedente is configured with Regime Forfettario (RF19). */
  readonly cedenteIsRf19 = computed(() => {
    const cedenteId = this.formState().cedenteId;
    if (!cedenteId) return false;
    const client = this.clientStore.clientMap().get(cedenteId);
    if (client?.regimeFiscale) {
      return client.regimeFiscale.toUpperCase() === "RF19";
    }
    const companyClientId = this.settingsStore.findCompanyClientId(this.clientStore.clients());
    return (
      cedenteId === companyClientId &&
      this.settingsStore.company()?.regime_fiscale?.toUpperCase() === "RF19"
    );
  });

  /** Live preview of the total amount for the line currently being composed in `lineDraftForm`. */
  readonly draftLinePreviewTotal = computed(() => {
    const draft = this.lineDraftState();
    const fields = this.composeDraftLineFields(draft);
    return lineAmount({
      quantita: draft.quantita ?? 1,
      prezzoUnitario: Number(draft.prezzoUnitario) || 0,
      fields,
    });
  });

  readonly draftLineRequiresNatura = computed(() => Number(this.lineDraftState().aliquotaIva) === 0);

  /** True when the user started typing a line in the bottom form but hasn't clicked Add/Update yet. */
  readonly hasUnsavedLineDraft = computed(() => {
    const draft = this.lineDraftState();
    return (draft.descrizione ?? "").trim().length > 0 || (Number(draft.prezzoUnitario) || 0) > 0;
  });

  /** Step 2 "Successivo" button is enabled only when at least one valid line has been added. */
  readonly canProceedFromLines = computed(() => {
    const lines = (this.formState().linee ?? []) as LineItemFormValue[];
    return lines.length > 0;
  });

  /** Computed running total of line items (taxable compensation) */
  readonly runningTotal = computed(() => {
    const state = this.formState();
    if (!state.linee) return 0;
    return (state.linee as LineItemFormValue[]).reduce((acc, line) => acc + lineAmount(line), 0);
  });

  /** Computed real-time breakdown of compensi, casse, iva, bollo, ritenute, and netto a pagare */
  readonly financialSummary = computed(() => {
    const state = this.formState();
    const rawLinee = (state.linee ?? []) as LineItemFormValue[];

    let imponibileLinee = 0;
    let nonTaxableTotal = 0;
    let bolloSubjectTotal = 0;
    const ivaMap = new Map<number, number>();

    for (const line of rawLinee) {
      const tot = lineAmount(line);
      imponibileLinee += tot;

      const al = Number(line?.aliquotaIva) || 0;
      if (al === 0) {
        nonTaxableTotal += tot;
        if (!line.natura || BOLLO_SUBJECT_NATURA_CODES.has(line.natura)) {
          bolloSubjectTotal += tot;
        }
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
      } else if (state.isForfettario || !state.cassaSoggettaIva) {
        bolloSubjectTotal += contributoCassa;
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

    // Bollo Virtuale (€2.00): mandatory over €77.47 for RF19 or exempt/excluded natures (D.M. 17/06/2014).
    // Respects explicit user opt-out (`applicaBollo === false` after manual uncheck) while surfacing a warning.
    const isExemptOver77 =
      (!!state.isForfettario && imponibileLinee + contributoCassa > 77.47) ||
      bolloSubjectTotal > 77.47;
    const bolloActive = !!state.applicaBollo;
    const bolloMissingWarning = isExemptOver77 && !bolloActive;
    const bolloAmt = bolloActive ? Number(state.importoBollo) || 2.0 : 0;

    // Totale Documento (Lordo)
    const bolloInLordo = bolloActive || !!state.riaddebitaBollo;
    const totaleLordo =
      Math.round((imponibileLinee + contributoCassa + totIva + (bolloInLordo ? bolloAmt : 0)) * 100) / 100;

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
    const nettoAPagare = Math.max(
      0,
      Math.round((totaleLordo - importoRitenuta - splitPaymentDeduction) * 100) / 100,
    );

    return {
      imponibileLinee,
      nonTaxableTotal,
      isExemptOver77,
      bolloMissingWarning,
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
    const td = this.formState().tipoDocumento;
    return ["TD16", "TD17", "TD18", "TD19", "TD20", "TD21", "TD27"].includes(td ?? "");
  });

  stepLabels = [
    "Dati Generali",
    "Linee Dettaglio",
    "Previdenza & Bollo",
    "Pagamento",
    "Riepilogo",
  ];

  readonly statutoryForfettarioPreview = STATUTORY_FORFETTARIO_CAUSALE_LINES.join(" ");

  constructor() {
    // Keep Natura cleared whenever a positive VAT rate is chosen in the line editor.
    this.lineDraftForm
      .get("aliquotaIva")!
      .valueChanges.pipe(takeUntilDestroyed(this.destroyRef))
      .subscribe((rate) => {
        const numericRate = Number(rate) || 0;
        if (numericRate > 0 && this.lineDraftForm.get("natura")?.value) {
          this.lineDraftForm.patchValue({ natura: "" }, { emitEvent: false });
        } else if (
          numericRate === 0 &&
          !this.lineDraftForm.get("natura")?.value &&
          this.form.get("isForfettario")?.value
        ) {
          this.lineDraftForm.patchValue({ natura: "N2.2" }, { emitEvent: false });
        }
      });

    // Automatically activate virtual stamp duty (> €77.47) unless manually unchecked by the user.
    this.linee.valueChanges.pipe(takeUntilDestroyed(this.destroyRef)).subscribe(() => {
      this.syncAutoBollo();
    });

    effect(async () => {
      const paramId = this.id();
      if (!paramId) return;

      this.invoiceId.set(paramId);
      try {
        const inv = await this.invoiceStore.getInvoice(paramId);
        this.loadedInvoice.set(inv);

        // Separate automatic statutory RF19 lines from user-written causale notes so they don't duplicate.
        const rawCausali = inv.causale ?? [];
        const hasStatutoryCausale = rawCausali.some((line) => line.includes("190/2014"));
        const userCausali = rawCausali.filter(
          (line) => !STATUTORY_FORFETTARIO_CAUSALE_LINES.includes(line.trim()),
        );

        this.form.patchValue({
          tipoDocumento: inv.tipoDocumento,
          numero: inv.numero,
          data: inv.data,
          divisa: inv.divisa,
          cedenteId: inv.cedenteId,
          cessionarioId: inv.cessionarioId,
          causale: userCausali.join("\n"),
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
          this.linee.push(this.createLineFormGroup({
            descrizione: line.descrizione,
            quantita: line.quantita,
            unitaMisura: line.unitaMisura ?? "",
            dataInizioPeriodo: line.dataInizioPeriodo ?? "",
            dataFinePeriodo: line.dataFinePeriodo ?? "",
            prezzoUnitario: line.prezzoUnitario,
            aliquotaIva: line.aliquotaIva,
            natura: line.natura ?? "",
            fields: this.lineFields(line),
          }));
        }

        // Restore Regime Forfettario state from Cedente RF19 or N2.2 lines
        const issuerRf19 =
          this.clientStore.clientMap().get(inv.cedenteId)?.regimeFiscale?.toUpperCase() === "RF19";
        const allLinesN22 =
          editableLines.length > 0 &&
          editableLines.every((l) => l.aliquotaIva === 0 && l.natura === "N2.2");
        if (issuerRf19 || allLinesN22 || hasStatutoryCausale) {
          this.form.patchValue({
            isForfettario: true,
            includiCausaleForfettario: hasStatutoryCausale || issuerRf19,
          });
          this.resetLineDraft(true);
        }

        if (inv.isPa || inv.formatoTrasmissione === "FPA12") {
          this.form.patchValue({ isPa: true });
        }
        if (inv.isSplitPayment || inv.esigibilitaIva === "S") {
          this.form.patchValue({ isSplitPayment: true });
        }
        const trac = inv.datiOrdineAcquisto?.[0] || inv.datiContratto?.[0] || inv.datiConvenzione?.[0];
        if (trac) {
          const tipoTrac = inv.datiContratto?.length
            ? "contratto"
            : inv.datiConvenzione?.length
              ? "convenzione"
              : "ordine";
          this.form.patchValue({
            tipoDocumentoTracciabilita: tipoTrac,
            idDocumentoTracciabilita: trac.idDocumento,
            dataDocumentoTracciabilita: trac.data ?? "",
            codiceCig: trac.codiceCig ?? "",
            codiceCup: trac.codiceCup ?? "",
          });
        }

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

        if (inv.datiRitenuta && inv.datiRitenuta.length > 0) {
          const rit = inv.datiRitenuta[0];
          this.form.patchValue({
            applicaRitenuta: true,
            tipoRitenuta: rit.tipoRitenuta,
            aliquotaRitenuta: rit.aliquotaRitenuta,
            causalePagamentoRitenuta: rit.causalePagamento,
          });
        }

        if (inv.datiBollo && inv.datiBollo.bolloVirtuale === "SI") {
          this.bolloManuallyDisabled.set(false);
          this.form.patchValue({
            applicaBollo: true,
            importoBollo: inv.datiBollo.importoBollo ?? 2.0,
          });
        } else if (this.financialSummary().isExemptOver77) {
          // Existing draft deliberately saved without bollo despite > €77.47
          this.bolloManuallyDisabled.set(true);
          this.form.patchValue({ applicaBollo: false });
        }

        if (inv.pagamenti?.length > 0) {
          const p = inv.pagamenti[0];
          this.paymentAmountCustomized.set(true);
          this.form.patchValue({
            condizioniPagamento: p.condizioniPagamento ?? "TP02",
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
          this.allegati.set(
            inv.allegati.map((a) => ({
              nomeAttachment: a.nomeAttachment,
              algoritmoCompressione: a.algoritmoCompressione,
              formatoAttachment: a.formatoAttachment,
              descrizioneAttachment: a.descrizioneAttachment,
              attachmentBase64: a.attachmentBase64,
            })),
          );
        }
      } catch (e) {
        this.notify.error(e instanceof Error ? e.message : "Errore nel caricamento della fattura");
        this.router.navigate(["/invoices"]);
      }
    });
  }

  async ngOnInit(): Promise<void> {
    try {
      await Promise.all([this.clientStore.loadAll(), this.settingsStore.load()]);
    } catch {
      this.notify.error("Errore nel caricamento delle anagrafiche");
    }

    if (!this.id()) {
      const navDraft = history.state?.autofatturaDraft as CreateInvoiceRequest | undefined;
      if (navDraft) {
        this.loadAutofatturaDraft(navDraft);
      } else {
        const defaultCedente = this.settingsStore.findCompanyClientId(this.clientStore.clients());
        if (defaultCedente) {
          this.form.patchValue({ cedenteId: defaultCedente });
          this.applyCedenteRegime(defaultCedente);
          void this.suggestNumber(defaultCedente);
        } else {
          this.resetLineDraft(false);
        }
      }

      this.form
        .get("cedenteId")!
        .valueChanges.pipe(takeUntilDestroyed(this.destroyRef))
        .subscribe((cedenteId) => {
          this.applyCedenteRegime(cedenteId);
          void this.suggestNumber(cedenteId);
        });
    }
  }

  /** Automatically aligns Regime Forfettario (RF19) and Bollo when the Cedente changes. */
  private applyCedenteRegime(cedenteId: string | null): void {
    if (!cedenteId || this.isAutofattura()) return;
    const client = this.clientStore.clientMap().get(cedenteId);
    const defaultCompanyId = this.settingsStore.findCompanyClientId(this.clientStore.clients());
    const regime =
      client?.regimeFiscale ??
      (cedenteId === defaultCompanyId ? this.settingsStore.company()?.regime_fiscale : null);
    const isRf19 = regime?.toUpperCase() === "RF19";
    if (isRf19 !== !!this.form.get("isForfettario")?.value) {
      this.toggleForfettario(isRf19);
    } else {
      this.syncAutoBollo();
    }
  }

  /** Automatically checks `applicaBollo` when legal €77.47 threshold is exceeded (unless manually unchecked). */
  syncAutoBollo(): void {
    const summary = this.financialSummary();
    if (summary.isExemptOver77) {
      if (!this.bolloManuallyDisabled() && !this.form.get("applicaBollo")?.value) {
        this.form.patchValue({ applicaBollo: true });
      }
    } else {
      this.bolloManuallyDisabled.set(false);
      if (this.form.get("applicaBollo")?.value && !this.form.get("riaddebitaBollo")?.value) {
        this.form.patchValue({ applicaBollo: false });
      }
    }
  }

  onBolloCheckboxChange(): void {
    const checked = !!this.form.get("applicaBollo")?.value;
    if (!checked && this.financialSummary().isExemptOver77) {
      this.bolloManuallyDisabled.set(true);
      this.form.patchValue({ riaddebitaBollo: false });
    } else if (checked) {
      this.bolloManuallyDisabled.set(false);
    }
  }

  onRiaddebitaBolloChange(): void {
    if (this.form.get("riaddebitaBollo")?.value) {
      this.bolloManuallyDisabled.set(false);
      this.form.patchValue({ applicaBollo: true });
    }
  }

  onPaymentAmountInput(): void {
    this.paymentAmountCustomized.set(true);
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
          isSplitPayment: !this.form.get("isForfettario")?.value,
        });
      }
    }
  }

  nextStep(): void {
    if (this.currentStep() === 2) {
      if (this.linee.length === 0) {
        this.notify.error("Aggiungi almeno una riga di dettaglio prima di proseguire.");
        return;
      }
      if (this.showDescriptionErrors()) return;
    }
    if (this.currentStep() < 5) {
      const next = this.currentStep() + 1;
      this.currentStep.set(next);
      if (next === 3) {
        this.syncAutoBollo();
      } else if (next === 4 && !this.paymentAmountCustomized()) {
        this.syncPaymentAmount();
      }
    }
  }

  private showDescriptionErrors(): boolean {
    const descriptions = this.linee.controls
      .map((line) => line.get("descrizione"))
      .filter((control) => control?.invalid);
    if (!descriptions.length) return false;

    for (const control of descriptions) control?.markAsTouched();
    this.currentStep.set(2);
    this.notify.error("Correggi le descrizioni delle righe evidenziate");
    return true;
  }

  prevStep(): void {
    if (this.currentStep() > 1) {
      this.currentStep.update((s) => s - 1);
    }
  }

  loadAutofatturaDraft(draft: CreateInvoiceRequest): void {
    const collegata = draft.datiFattureCollegate?.[0];
    this.paymentAmountCustomized.set(true);
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
      condizioniPagamento: draft.pagamenti?.[0]?.condizioniPagamento ?? "TP02",
      modalitaPagamento: draft.pagamenti?.[0]?.modalitaPagamento ?? "MP05",
      importoPagamento: draft.pagamenti?.[0]?.importoPagamento ?? 0,
    });

    this.linee.clear();
    for (const l of draft.linee) {
      this.linee.push(
        this.createLineFormGroup({
          descrizione: l.descrizione,
          quantita: l.quantita,
          unitaMisura: l.unitaMisura ?? "",
          dataInizioPeriodo: l.dataInizioPeriodo ?? "",
          dataFinePeriodo: l.dataFinePeriodo ?? "",
          prezzoUnitario: l.prezzoUnitario,
          aliquotaIva: l.aliquotaIva,
          natura: l.natura ?? "",
          fields: this.lineFields(l),
        }),
      );
    }
  }

  private lineFields(line: InvoiceLineFields): InvoiceLineFields {
    return {
      tipoCessionePrestazione: line.tipoCessionePrestazione,
      codiceArticolo: line.codiceArticolo,
      scontoMaggiorazione: line.scontoMaggiorazione,
      ritenuta: line.ritenuta,
      riferimentoAmministrazione: line.riferimentoAmministrazione,
      altriDatiGestionali: line.altriDatiGestionali,
    };
  }

  private createLineFormGroup(item: Partial<LineItemFormValue> = {}): FormGroup {
    const isForfettario = !!this.form.get("isForfettario")?.value;
    const defaultIva = isForfettario ? 0 : 22;
    const aliquota = item.aliquotaIva ?? defaultIva;
    const defaultNatura = aliquota === 0 && isForfettario ? "N2.2" : "";
    return this.fb.group({
      descrizione: [item.descrizione ?? "", sdiDescriptionValidator],
      quantita: [item.quantita !== undefined ? item.quantita : 1],
      unitaMisura: [item.unitaMisura ?? "pz"],
      dataInizioPeriodo: [item.dataInizioPeriodo ?? ""],
      dataFinePeriodo: [item.dataFinePeriodo ?? ""],
      prezzoUnitario: [item.prezzoUnitario ?? 0, Validators.required],
      aliquotaIva: [aliquota, Validators.required],
      natura: [aliquota === 0 ? item.natura ?? defaultNatura : ""],
      fields: [this.lineFields(item.fields ?? {})],
    });
  }

  private composeDraftLineFields(
    draft: Partial<ReturnType<typeof this.lineDraftForm.getRawValue>>,
  ): InvoiceLineFields {
    const base = this.lineFields(draft.fields ?? {});
    const val = Number(draft.scontoValore);
    if (draft.scontoTipo && Number.isFinite(val) && val > 0) {
      const first =
        draft.scontoModo === "importo"
          ? { tipo: draft.scontoTipo, importo: val, percentuale: null }
          : { tipo: draft.scontoTipo, percentuale: val, importo: null };
      const extra = base.scontoMaggiorazione?.slice(1) ?? [];
      return { ...base, scontoMaggiorazione: [first, ...extra] };
    }
    if (draft.scontoTipo === "") {
      const extra = base.scontoMaggiorazione?.slice(1) ?? [];
      return { ...base, scontoMaggiorazione: extra.length > 0 ? extra : undefined };
    }
    return base;
  }

  private resetLineDraft(forfettario = !!this.form.get("isForfettario")?.value): void {
    this.customIvaRate.set(null);
    this.lineDraftForm.reset({
      descrizione: "",
      quantita: 1,
      unitaMisura: "pz",
      prezzoUnitario: 0,
      aliquotaIva: forfettario ? 0 : 22,
      natura: forfettario ? "N2.2" : "",
      dataInizioPeriodo: "",
      dataFinePeriodo: "",
      scontoTipo: "",
      scontoModo: "percentuale",
      scontoValore: null,
      fields: {},
    });
  }

  /** Validates `lineDraftForm` and appends or updates the line in `linee`. */
  saveLineDraft(): boolean {
    const raw = this.lineDraftForm.getRawValue();
    this.lineDraftForm.markAllAsTouched();
    if (this.lineDraftForm.invalid) {
      this.notify.error("Controlla la descrizione e il prezzo unitario della riga.");
      return false;
    }
    const aliquotaIva = Number(raw.aliquotaIva) || 0;
    const natura = aliquotaIva === 0 ? (raw.natura || "").trim() : "";
    if (aliquotaIva === 0 && !natura) {
      this.notify.error("Per le righe con IVA 0% è obbligatorio selezionare la Natura IVA (es. N2.2).");
      return false;
    }
    if (raw.dataInizioPeriodo && raw.dataFinePeriodo && raw.dataFinePeriodo < raw.dataInizioPeriodo) {
      this.notify.error("La data di fine periodo non può precedere la data di inizio periodo.");
      return false;
    }

    const lineValue: LineItemFormValue = {
      descrizione: (raw.descrizione ?? "").trim(),
      quantita: raw.quantita === null || raw.quantita === undefined ? null : Number(raw.quantita),
      unitaMisura: (raw.unitaMisura ?? "").trim(),
      dataInizioPeriodo: raw.dataInizioPeriodo || "",
      dataFinePeriodo: raw.dataFinePeriodo || "",
      prezzoUnitario: Number(raw.prezzoUnitario) || 0,
      aliquotaIva,
      natura,
      fields: this.composeDraftLineFields(raw),
    };

    const editingIdx = this.editingLineIndex();
    if (editingIdx !== null && editingIdx >= 0 && editingIdx < this.linee.length) {
      this.linee.at(editingIdx).patchValue(lineValue);
      this.editingLineIndex.set(null);
    } else {
      this.linee.push(this.createLineFormGroup(lineValue));
    }

    this.resetLineDraft();
    this.syncAutoBollo();
    if (!this.paymentAmountCustomized()) {
      this.syncPaymentAmount();
    }
    return true;
  }

  /** Loads an existing row into the bottom editor so the user can modify all its fields. */
  editLine(index: number): void {
    const group = this.linee.at(index) as FormGroup | undefined;
    if (!group) return;
    const val = group.getRawValue() as LineItemFormValue;
    const firstAdj = val.fields?.scontoMaggiorazione?.[0];
    this.customIvaRate.set(Number(val.aliquotaIva));
    this.editingLineIndex.set(index);
    this.lineDraftForm.reset({
      descrizione: val.descrizione,
      quantita: val.quantita,
      unitaMisura: val.unitaMisura ?? "",
      prezzoUnitario: val.prezzoUnitario,
      aliquotaIva: Number(val.aliquotaIva),
      natura: val.natura ?? "",
      dataInizioPeriodo: val.dataInizioPeriodo ?? "",
      dataFinePeriodo: val.dataFinePeriodo ?? "",
      scontoTipo: firstAdj?.tipo ?? "",
      scontoModo: firstAdj?.importo != null ? "importo" : "percentuale",
      scontoValore: firstAdj?.importo ?? firstAdj?.percentuale ?? null,
      fields: this.lineFields(val.fields ?? {}),
    });
  }

  cancelEditLine(): void {
    this.editingLineIndex.set(null);
    this.resetLineDraft();
  }

  /** Programmatic line helper (preserves unit test and quick-append ergonomics). */
  addLine(preset?: Partial<LineItemFormValue>): void {
    this.linee.push(this.createLineFormGroup(preset));
    this.syncAutoBollo();
  }

  removeLine(index: number): void {
    const editing = this.editingLineIndex();
    if (editing === index) {
      this.cancelEditLine();
    } else if (editing !== null && editing > index) {
      this.editingLineIndex.set(editing - 1);
    }
    this.linee.removeAt(index);
    this.syncAutoBollo();
  }

  lineTotal(index: number): number {
    const line = this.linee.at(index) as FormGroup;
    return lineAmount(line.getRawValue() as LineItemFormValue);
  }

  naturaLabel(code: string | null | undefined): string {
    if (!code) return "";
    return NATURA[code] ? `${code} — ${NATURA[code]}` : code;
  }

  computeTotal(): number {
    return this.runningTotal();
  }

  syncPaymentAmount(): void {
    const summary = this.financialSummary();
    this.form.patchValue({ importoPagamento: summary.nettoAPagare });
  }

  toggleForfettario(enabled: boolean): void {
    this.form.patchValue({
      isForfettario: enabled,
      includiCausaleForfettario: enabled,
    });
    if (enabled) {
      // 1. All existing lines move to 0% VAT with Natura N2.2
      for (let i = 0; i < this.linee.length; i++) {
        const line = this.linee.at(i);
        line.patchValue({
          aliquotaIva: 0,
          natura: "N2.2",
        });
      }
      // 2. Align the bottom line editor defaults as well
      if (this.editingLineIndex() === null) {
        this.lineDraftForm.patchValue({ aliquotaIva: 0, natura: "N2.2" });
      }
      // 3. No withholding tax or Split Payment VAT in RF19
      this.form.patchValue({
        applicaRitenuta: false,
        cassaSoggettaIva: false,
        isSplitPayment: false,
      });
    } else if (this.editingLineIndex() === null && Number(this.lineDraftForm.get("aliquotaIva")?.value) === 0) {
      this.lineDraftForm.patchValue({ aliquotaIva: 22, natura: "" });
    }
    this.syncAutoBollo();
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
    const linee: CreateLineRequest[] = rawLinee.map((l) => {
      const aliquota = Number(l.aliquotaIva) || 0;
      return {
        ...l.fields,
        descrizione: l.descrizione,
        quantita: l.quantita ?? null,
        unitaMisura: l.unitaMisura || null,
        dataInizioPeriodo: l.dataInizioPeriodo || null,
        dataFinePeriodo: l.dataFinePeriodo || null,
        prezzoUnitario: Number(l.prezzoUnitario) || 0,
        aliquotaIva: aliquota,
        natura: aliquota === 0 ? l.natura || (val.isForfettario ? "N2.2" : null) : null,
      };
    });

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
          natura: val.isForfettario || !val.cassaSoggettaIva ? (val.isForfettario ? "N2.2" : "N4") : null,
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

    // Build Bollo Virtuale strictly when `applicaBollo` is enabled
    let datiBollo: DatiBolloDetail | null = null;
    if (val.applicaBollo && summary.bolloAmt > 0) {
      datiBollo = {
        bolloVirtuale: "SI",
        importoBollo: summary.bolloAmt,
      };
    }

    // Parse causale lines into <=200-char chunks, appending statutory RF19 note only on export/save if requested
    const causaleLines = val.causale ? splitCausaleChunks(val.causale) : [];
    if (
      val.isForfettario &&
      val.includiCausaleForfettario &&
      !causaleLines.some((line) => line.includes("190/2014"))
    ) {
      causaleLines.push(...STATUTORY_FORFETTARIO_CAUSALE_LINES);
    }

    const condizioniPagamento = val.condizioniPagamento || "TP02";
    const extraPagamenti = (orig?.pagamenti?.slice(1) ?? []).map((p) => ({
      condizioniPagamento: p.condizioniPagamento ?? condizioniPagamento,
      modalitaPagamento: p.modalitaPagamento,
      importoPagamento: p.importoPagamento,
      dataScadenzaPagamento: p.dataScadenzaPagamento,
      iban: p.iban,
    }));

    const extraCollegate = orig?.datiFattureCollegate?.slice(1) ?? [];
    const effectivePaymentAmount =
      !this.paymentAmountCustomized() && (Number(val.importoPagamento) || 0) === 0
        ? summary.nettoAPagare
        : Number(val.importoPagamento) || 0;

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
          condizioniPagamento,
          modalitaPagamento: val.modalitaPagamento!,
          importoPagamento: effectivePaymentAmount,
          dataScadenzaPagamento: val.dataScadenzaPagamento || null,
          iban: val.iban?.trim() || null,
        },
        ...extraPagamenti,
      ],
      datiCassaPrevidenziale,
      datiRitenuta,
      datiBollo,
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
      datiOrdineAcquisto:
        (val.idDocumentoTracciabilita || val.codiceCig || val.codiceCup) &&
        val.tipoDocumentoTracciabilita === "ordine"
          ? [
              {
                tipoDocumentoRif: "ordine",
                idDocumento:
                  val.idDocumentoTracciabilita || (val.codiceCig ? `CIG-${val.codiceCig}` : "ORD-1"),
                data: val.dataDocumentoTracciabilita || null,
                codiceCig: val.codiceCig ? val.codiceCig.trim() : null,
                codiceCup: val.codiceCup ? val.codiceCup.trim() : null,
              },
              ...(orig?.datiOrdineAcquisto?.slice(1) ?? []),
            ]
          : null,
      datiContratto:
        (val.idDocumentoTracciabilita || val.codiceCig || val.codiceCup) &&
        val.tipoDocumentoTracciabilita === "contratto"
          ? [
              {
                tipoDocumentoRif: "contratto",
                idDocumento:
                  val.idDocumentoTracciabilita || (val.codiceCig ? `CIG-${val.codiceCig}` : "CONTR-1"),
                data: val.dataDocumentoTracciabilita || null,
                codiceCig: val.codiceCig ? val.codiceCig.trim() : null,
                codiceCup: val.codiceCup ? val.codiceCup.trim() : null,
              },
              ...(orig?.datiContratto?.slice(1) ?? []),
            ]
          : null,
      datiConvenzione:
        (val.idDocumentoTracciabilita || val.codiceCig || val.codiceCup) &&
        val.tipoDocumentoTracciabilita === "convenzione"
          ? [
              {
                tipoDocumentoRif: "convenzione",
                idDocumento:
                  val.idDocumentoTracciabilita || (val.codiceCig ? `CIG-${val.codiceCig}` : "CONV-1"),
                data: val.dataDocumentoTracciabilita || null,
                codiceCig: val.codiceCig ? val.codiceCig.trim() : null,
                codiceCup: val.codiceCup ? val.codiceCup.trim() : null,
              },
              ...(orig?.datiConvenzione?.slice(1) ?? []),
            ]
          : null,
      datiDdt: this.datiDdt().length > 0 ? this.datiDdt() : null,
      allegati: this.allegati().length > 0 ? this.allegati() : null,
    };
  }

  private checkBeforePersist(): boolean {
    if (!this.form.get("numero")?.value?.trim()) {
      this.currentStep.set(1);
      this.form.get("numero")?.markAsTouched();
      this.notify.error("Inserisci il numero della fattura nello Step 1.");
      return false;
    }
    if (!this.form.get("cedenteId")?.value) {
      this.currentStep.set(1);
      this.form.get("cedenteId")?.markAsTouched();
      this.notify.error("Seleziona il Cedente (fornitore) nello Step 1.");
      return false;
    }
    if (!this.form.get("cessionarioId")?.value) {
      this.currentStep.set(1);
      this.form.get("cessionarioId")?.markAsTouched();
      this.notify.error("Seleziona il Cessionario (cliente) nello Step 1.");
      return false;
    }
    if (this.linee.length === 0) {
      this.currentStep.set(2);
      this.notify.error("Inserisci almeno una riga di dettaglio nello Step 2 prima di salvare.");
      return false;
    }
    if (this.showDescriptionErrors()) {
      return false;
    }
    if (this.form.invalid) {
      this.form.markAllAsTouched();
      this.notify.error("Controlla i campi obbligatori evidenziati nel modulo.");
      return false;
    }
    return true;
  }

  private async saveDraft(): Promise<string> {
    const request = this.buildRequest();
    const editId = this.invoiceId();
    if (this.isEditMode() && editId) {
      const updated = await this.invoiceStore.updateInvoice(
        editId,
        request,
        this.loadedInvoice()?.revision,
      );
      const nextId = updated?.id || editId;
      if (updated) this.loadedInvoice.set(updated);
      this.invoiceId.set(nextId);
      return nextId;
    }
    const created = await this.invoiceStore.createInvoice(request);
    this.loadedInvoice.set(created);
    this.invoiceId.set(created.id);
    return created.id;
  }

  async validateDraft(): Promise<void> {
    if (!this.checkBeforePersist()) return;
    this.validating.set(true);
    try {
      const id = await this.saveDraft();
      const result = await this.invoiceStore.validateInvoice(id);
      this.validationErrors.set(result.errors);
      this.validated.set(result.valid);
      if (result.valid) {
        this.notify.success("Controlli locali SdI superati");
      } else {
        this.notify.error(`Validazione SdI non superata: ${result.errors.length} rilievi da correggere`);
      }
    } catch (e) {
      this.notify.error(e instanceof Error ? e.message : "Errore durante la validazione della fattura");
    } finally {
      this.validating.set(false);
    }
  }

  async onSubmit(): Promise<void> {
    if (!this.checkBeforePersist()) return;
    try {
      const editId = this.invoiceId();
      await this.saveDraft();
      this.notify.success(this.isEditMode() && editId ? "Fattura aggiornata" : "Bozza fattura salvata");
      this.router.navigate(["/invoices"]);
    } catch (e) {
      this.notify.error(e instanceof Error ? e.message : "Errore durante il salvataggio della fattura");
    }
  }

  cancel(): void {
    this.router.navigate(["/invoices"]);
  }
}
