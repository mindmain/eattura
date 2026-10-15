import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  OnDestroy,
  computed,
  effect,
  inject,
  signal,
  viewChild,
} from "@angular/core";
import { Router, RouterLink } from "@angular/router";
import { Chart, registerables } from "chart.js";
import { InvoiceStore } from "../../core/stores/invoice.store";
import { ClientStore } from "../../core/stores/client.store";
import { AppCurrencyPipe } from "../../shared/pipes/app-currency.pipe";
import { ThemeService } from "../../core/services/theme.service";

Chart.register(...registerables);

/** Monthly aggregation (12 entries, Jan..Dec) for a single year. */
interface MonthlyStats {
  income: number[];
  cost: number[];
  counts: number[];
}

const MONTH_LABELS = [
  "Gen", "Feb", "Mar", "Apr", "Mag", "Giu",
  "Lug", "Ago", "Set", "Ott", "Nov", "Dic",
];

@Component({
  selector: "app-dashboard",
  standalone: true,
  imports: [RouterLink, AppCurrencyPipe],
  templateUrl: "./dashboard.component.html",
  styleUrl: "./dashboard.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DashboardComponent implements OnDestroy {
  readonly invoiceStore = inject(InvoiceStore);
  readonly clientStore = inject(ClientStore);
  private router = inject(Router);
  private readonly theme = inject(ThemeService);

  readonly chartCanvas = viewChild<ElementRef<HTMLCanvasElement>>("yearChart");
  readonly annualChartCanvas = viewChild<ElementRef<HTMLCanvasElement>>("annualChart");
  private chart?: Chart;
  private annualChart?: Chart;
  private readonly currentYear = new Date().getFullYear();

  readonly annualYears = Array.from(
    { length: 8 },
    (_, index) => String(this.currentYear - 7 + index),
  );

  // Expose store metrics directly to template
  readonly totalInvoices = this.invoiceStore.totalCount;
  readonly draftCount = this.invoiceStore.draftCount;
  readonly sentCount = this.invoiceStore.sentCount;
  readonly clientCount = this.clientStore.clientCount;

  /** Monthly stats grouped by year derived from invoices. */
  readonly monthlyByYear = computed(() => {
    const invoices = this.invoiceStore.invoices();
    const map = new Map<string, MonthlyStats>();

    for (const inv of invoices) {
      if (!inv.data || inv.data.length < 7) continue;
      const year = inv.data.substring(0, 4);
      const month = Number(inv.data.substring(5, 7)) - 1; // 0..11
      if (Number.isNaN(month) || month < 0 || month > 11) continue;

      let stats = map.get(year);
      if (!stats) {
        stats = {
          income: new Array(12).fill(0),
          cost: new Array(12).fill(0),
          counts: new Array(12).fill(0),
        };
        map.set(year, stats);
      }

      const total = inv.importoTotale ?? 0;
      stats.counts[month] += 1;
      if (inv.direction === "sale") {
        stats.income[month] += total;
      } else if (inv.direction === "purchase") {
        stats.cost[month] += total;
      }
    }
    return map;
  });

  /** Available years (descending) shown in the select. */
  readonly years = computed(() => {
    return Array.from(this.monthlyByYear().keys()).sort((a, b) => b.localeCompare(a));
  });

  /** Currently selected year. */
  selectedYear = signal<string>("");

  /** Income / cost totals for the selected year computed reactively. */
  readonly yearIncome = computed(() => {
    const year = this.selectedYear();
    const stats = this.monthlyByYear().get(year);
    return stats ? sum(stats.income) : 0;
  });

  readonly yearCost = computed(() => {
    const year = this.selectedYear();
    const stats = this.monthlyByYear().get(year);
    return stats ? sum(stats.cost) : 0;
  });

  readonly annualIncome = computed(() => {
    const monthlyMap = this.monthlyByYear();
    return this.annualYears.map((year) => sum(monthlyMap.get(year)?.income ?? []));
  });

  constructor() {
    // Initial data load
    void Promise.all([
      this.invoiceStore.loadAll(),
      this.clientStore.loadAll(),
    ]);

    // Automatically pick default year when years list changes
    effect(() => {
      const available = this.years();
      const current = this.selectedYear();
      if (available.length > 0 && (!current || !available.includes(current))) {
        const thisYear = String(new Date().getFullYear());
        this.selectedYear.set(available.includes(thisYear) ? thisYear : available[0]);
      }
    });

    // Render / update chart whenever chartCanvas or selectedYear or data changes
    effect((onCleanup) => {
      const canvasRef = this.chartCanvas();
      const year = this.selectedYear();
      const monthlyMap = this.monthlyByYear();
      this.theme.effectiveTheme();

      if (!canvasRef || !year) return;

      const stats = monthlyMap.get(year) ?? {
        income: new Array(12).fill(0),
        cost: new Array(12).fill(0),
        counts: new Array(12).fill(0),
      };

      const chartInstance = this.renderChart(canvasRef.nativeElement, stats);

      onCleanup(() => {
        chartInstance.destroy();
      });
    });

    effect((onCleanup) => {
      const canvasRef = this.annualChartCanvas();
      const income = this.annualIncome();
      this.theme.effectiveTheme();
      if (!canvasRef) return;

      const chartInstance = this.renderAnnualChart(canvasRef.nativeElement, income);
      onCleanup(() => chartInstance.destroy());
    });
  }

  ngOnDestroy(): void {
    this.chart?.destroy();
    this.annualChart?.destroy();
  }

  /** Handle the year <select> change. */
  onYearChange(event: Event): void {
    this.selectedYear.set((event.target as HTMLSelectElement).value);
  }

  /** Select a year. */
  selectYear(year: string): void {
    this.selectedYear.set(year);
  }

  /** Open the invoices list filtered to the selected year. */
  viewYearInvoices(): void {
    const year = this.selectedYear();
    if (year) {
      this.router.navigate(["/invoices"], { queryParams: { year } });
    }
  }

  /** Open the invoices list filtered to a specific month of the selected year. */
  private viewMonthInvoices(monthIndex: number): void {
    const year = this.selectedYear();
    if (!year) return;
    const month = String(monthIndex + 1).padStart(2, "0");
    this.router.navigate(["/invoices"], { queryParams: { year, month } });
  }

  private renderChart(canvas: HTMLCanvasElement, stats: MonthlyStats): Chart {
    if (this.chart) {
      this.chart.destroy();
    }

    const self = this;
    const textColor = this.theme.effectiveTheme() === "dark" ? "#d0d5dc" : "#34465f";
    const gridColor = this.theme.effectiveTheme() === "dark" ? "#2b3038" : "#dce3ee";
    const currencyFormatter = new Intl.NumberFormat("it-IT", {
      style: "currency",
      currency: "EUR",
      maximumFractionDigits: 0,
    });

    const chart = new Chart(canvas, {
      type: "bar",
      data: {
        labels: MONTH_LABELS,
        datasets: [
          {
            label: "Fatturato (Vendite)",
            data: stats.income,
            backgroundColor: "#2e7d32",
            borderRadius: 4,
          },
          {
            label: "Costi (Acquisti)",
            data: stats.cost,
            backgroundColor: "#c62828",
            borderRadius: 4,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: {
            position: "top",
            labels: { color: textColor },
          },
          tooltip: {
            callbacks: {
              label: (context) => {
                const label = context.dataset.label ?? "";
                const val = Number(context.raw ?? 0);
                return ` ${label}: ${currencyFormatter.format(val)}`;
              },
            },
          },
        },
        onClick: (_event, elements) => {
          if (elements.length > 0) {
            const monthIndex = elements[0].index;
            self.viewMonthInvoices(monthIndex);
          }
        },
        scales: {
          y: {
            beginAtZero: true,
            grid: { color: gridColor },
            ticks: {
              color: textColor,
              callback: (val) => `${Number(val).toLocaleString("it-IT")} €`,
            },
          },
          x: { ticks: { color: textColor }, grid: { color: gridColor } },
        },
      },
    });

    this.chart = chart;
    return chart;
  }

  private renderAnnualChart(canvas: HTMLCanvasElement, income: number[]): Chart {
    this.annualChart?.destroy();
    const textColor = this.theme.effectiveTheme() === "dark" ? "#d0d5dc" : "#34465f";
    const gridColor = this.theme.effectiveTheme() === "dark" ? "#2b3038" : "#dce3ee";

    const currencyFormatter = new Intl.NumberFormat("it-IT", {
      style: "currency",
      currency: "EUR",
      maximumFractionDigits: 0,
    });

    const chart = new Chart(canvas, {
      type: "line",
      data: {
        labels: this.annualYears,
        datasets: [{
          label: "Fatturato",
          data: income,
          borderColor: "#2563eb",
          backgroundColor: "rgba(37, 99, 235, 0.14)",
          borderWidth: 3,
          fill: true,
          tension: 0.35,
          pointRadius: 5,
          pointHoverRadius: 7,
          pointBackgroundColor: "#2563eb",
          pointBorderColor: "#fff",
          pointBorderWidth: 2,
        }],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { display: false },
          tooltip: {
            callbacks: {
              label: (context) => ` Fatturato: ${currencyFormatter.format(Number(context.raw ?? 0))}`,
            },
          },
        },
        onClick: (_event, elements) => {
          if (elements.length > 0) {
            const year = this.annualYears[elements[0].index];
            void this.router.navigate(["/invoices"], { queryParams: { year } });
          }
        },
        scales: {
          y: {
            beginAtZero: true,
            grid: { color: gridColor },
            ticks: {
              color: textColor,
              callback: (val) => `${Number(val).toLocaleString("it-IT")} €`,
            },
          },
          x: { ticks: { color: textColor }, grid: { color: gridColor } },
        },
      },
    });

    this.annualChart = chart;
    return chart;
  }
}

function sum(values: number[]): number {
  return values.reduce((acc, v) => acc + v, 0);
}
