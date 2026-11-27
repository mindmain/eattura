import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  OnInit,
  signal,
} from "@angular/core";
import { CommonModule } from "@angular/common";
import { FormBuilder, FormGroup, ReactiveFormsModule, Validators } from "@angular/forms";
import { PaymentService } from "../../core/services/payment.service";
import { NotificationService } from "../../core/services/notification.service";
import type {
  PaymentScheduleItem,
  ScheduleFilter,
  IvaLiquidationResult,
  RegisterPaymentRequest,
} from "../../core/models/payment.model";
import { AppCurrencyPipe } from "../../shared/pipes/app-currency.pipe";
import { DatePickerComponent } from "../../shared/components/date-picker/date-picker.component";

@Component({
  selector: "app-schedule",
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule, AppCurrencyPipe, DatePickerComponent],
  templateUrl: "./schedule.component.html",
  styleUrl: "./schedule.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ScheduleComponent implements OnInit {
  private paymentService = inject(PaymentService);
  private notify = inject(NotificationService);
  private fb = inject(FormBuilder);

  readonly currentYear = new Date().getFullYear();

  activeTab = signal<"schedule" | "iva">("schedule");
  loading = signal<boolean>(false);
  ivaLoading = signal<boolean>(false);

  // --- Scadenzario State ---
  scheduleItems = signal<PaymentScheduleItem[]>([]);
  directionFilter = signal<"all" | "outbound" | "inbound">("all");
  statusFilter = signal<"all" | "scaduta" | "in_scadenza" | "futura" | "saldata">("all");
  selectedItemForPayment = signal<PaymentScheduleItem | null>(null);

  // --- Liquidazione IVA State ---
  liquidationResult = signal<IvaLiquidationResult | null>(null);

  // --- Forms ---
  ivaForm: FormGroup = this.fb.group({
    year: [this.currentYear, [Validators.required, Validators.min(2000)]],
    period: ["Q1", Validators.required],
    isTrimestrale: [true],
    creditoPrecedente: [0, [Validators.min(0)]],
  });

  paymentForm: FormGroup = this.fb.group({
    dataPagamento: [new Date().toISOString().substring(0, 10), Validators.required],
    importo: [0, [Validators.required, Validators.min(0.01)]],
    modalita: ["MP05", Validators.required],
    saldaTutto: [true],
    note: [""],
  });

  // --- Computed Metrics & Filtering ---
  filteredSchedule = computed(() => {
    const items = this.scheduleItems();
    const dir = this.directionFilter();
    const status = this.statusFilter();

    return items.filter((item) => {
      const matchDir = dir === "all" || item.direction === dir;
      const matchStatus = status === "all" || item.statoScadenza === status;
      return matchDir && matchStatus;
    });
  });

  metrics = computed(() => {
    const items = this.scheduleItems();
    let totalScaduto = 0;
    let countScaduto = 0;
    let totalInScadenza = 0;
    let countInScadenza = 0;
    let totalCreditiAttivi = 0;
    let totalDebitiPassivi = 0;

    for (const item of items) {
      if (item.statoScadenza === "scaduta") {
        totalScaduto += item.saldoResiduo;
        countScaduto += 1;
      } else if (item.statoScadenza === "in_scadenza") {
        totalInScadenza += item.saldoResiduo;
        countInScadenza += 1;
      }

      if (item.direction === "outbound") {
        totalCreditiAttivi += item.saldoResiduo;
      } else if (item.direction === "inbound") {
        totalDebitiPassivi += item.saldoResiduo;
      }
    }

    return {
      totalScaduto,
      countScaduto,
      totalInScadenza,
      countInScadenza,
      totalCreditiAttivi,
      totalDebitiPassivi,
    };
  });

  async ngOnInit(): Promise<void> {
    await Promise.all([this.loadSchedule(), this.calculateIva()]);
  }

  setDirectionFilter(dir: "all" | "outbound" | "inbound"): void {
    this.directionFilter.set(dir);
  }

  setStatusFilter(status: "all" | "scaduta" | "in_scadenza" | "futura" | "saldata"): void {
    this.statusFilter.set(status);
  }

  async loadSchedule(): Promise<void> {
    this.loading.set(true);
    try {
      const filter: ScheduleFilter = {
        direction: this.directionFilter(),
        stato: this.statusFilter(),
      };
      const items = await this.paymentService.getPaymentSchedule(filter);
      this.scheduleItems.set(items);
    } catch (e: unknown) {
      this.notify.error(`Errore caricamento scadenzario: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      this.loading.set(false);
    }
  }

  openPaymentModal(item: PaymentScheduleItem): void {
    this.selectedItemForPayment.set(item);
    this.paymentForm.patchValue({
      dataPagamento: new Date().toISOString().substring(0, 10),
      importo: item.saldoResiduo,
      modalita: "MP05",
      saldaTutto: true,
      note: "",
    });
  }

  closePaymentModal(): void {
    this.selectedItemForPayment.set(null);
  }

  async submitPayment(): Promise<void> {
    const item = this.selectedItemForPayment();
    if (!item || this.paymentForm.invalid) {
      this.paymentForm.markAllAsTouched();
      return;
    }

    const formVal = this.paymentForm.value;
    const req: RegisterPaymentRequest = {
      invoiceId: item.invoiceId,
      dataPagamento: formVal.dataPagamento,
      importo: Number(formVal.importo),
      modalita: formVal.modalita,
      saldaTutto: Boolean(formVal.saldaTutto),
      note: formVal.note || undefined,
    };

    try {
      await this.paymentService.registerPayment(req);
      this.notify.success("Pagamento registrato con successo");
      this.closePaymentModal();
      await this.loadSchedule();
    } catch (e: unknown) {
      this.notify.error(`Errore salvataggio pagamento: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  onPeriodTypeChange(isTrimestrale: boolean): void {
    this.ivaForm.patchValue({
      isTrimestrale,
      period: isTrimestrale ? "Q1" : "M01",
    });
    this.calculateIva();
  }

  async calculateIva(): Promise<void> {
    if (this.ivaForm.invalid) return;

    this.ivaLoading.set(true);
    try {
      const val = this.ivaForm.value;
      const result = await this.paymentService.calculateIvaLiquidation({
        year: Number(val.year),
        period: val.period,
        isTrimestrale: Boolean(val.isTrimestrale),
        creditoPrecedente: Number(val.creditoPrecedente) || 0,
      });
      this.liquidationResult.set(result);
    } catch (e: unknown) {
      this.notify.error(`Errore calcolo IVA: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      this.ivaLoading.set(false);
    }
  }
}
