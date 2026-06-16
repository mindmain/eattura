import {
  AfterViewInit,
  Component,
  ElementRef,
  OnDestroy,
  ViewChild,
  inject,
  signal,
} from "@angular/core";
import { Router, RouterLink } from "@angular/router";
import { Chart, registerables } from "chart.js";
import { InvoiceService } from "../../core/services/invoice.service";
import { ClientService } from "../../core/services/client.service";
import type { InvoiceSummary } from "../../core/models/invoice.model";

Chart.register(...registerables);

/** Monthly aggregation (12 entries, Jan..Dec) for a single year. */
interface MonthlyStats {
  /** Income per month (owner is cedente). */
  income: number[];
  /** Cost per month (owner is cessionario). */
  cost: number[];
  /** Invoice count per month (income + cost). */
  counts: number[];
}

/** Italian short month labels, January first. */
const MONTH_LABELS = [
  "Gen", "Feb", "Mar", "Apr", "Mag", "Giu",
  "Lug", "Ago", "Set", "Ott", "Nov", "Dic",
];

@Component({
  selector: "app-dashboard",
  standalone: true,
  imports: [RouterLink],
  templateUrl: "./dashboard.component.html",
  styleUrl: "./dashboard.component.css",
})
export class DashboardComponent implements AfterViewInit, OnDestroy {
  private invoiceService = inject(InvoiceService);
  private clientService = inject(ClientService);
  private router = inject(Router);

  @ViewChild("yearChart") private chartCanvas?: ElementRef<HTMLCanvasElement>;
  private chart?: Chart;

  /** Monthly stats keyed by year. */
  private monthlyByYear = new Map<string, MonthlyStats>();

  totalInvoices = signal(0);
  draftCount = signal(0);
  sentCount = signal(0);
  clientCount = signal(0);

  /** Available years (descending) shown in the select. */
  years = signal<string[]>([]);
  /** Currently selected year. */
  selectedYear = signal<string>("");
  /** Income / cost totals for the selected year. */
  yearIncome = signal(0);
  yearCost = signal(0);

  async ngAfterViewInit(): Promise<void> {
    try {
      const [invoices, clients] = await Promise.all([
        this.invoiceService.listInvoices(),
        this.clientService.listClients(),
      ]);

      this.totalInvoices.set(invoices.length);
      this.draftCount.set(invoices.filter((i) => i.stato === "draft").length);
      this.sentCount.set(invoices.filter((i) => i.stato === "sent").length);
      this.clientCount.set(clients.length);

      this.aggregateByMonth(invoices);
      const years = Array.from(this.monthlyByYear.keys()).sort((a, b) => b.localeCompare(a));
      this.years.set(years);

      if (years.length > 0) {
        // Default to the current year when present, otherwise the most recent.
        const current = String(new Date().getFullYear());
        this.selectYear(years.includes(current) ? current : years[0]);
      }
    } catch {
      // Stats remain at 0 / chart stays empty if the backend is unavailable.
    }
  }

  ngOnDestroy(): void {
    this.chart?.destroy();
  }

  /** Handle the year <select> change. */
  onYearChange(event: Event): void {
    this.selectYear((event.target as HTMLSelectElement).value);
  }

  /** Select a year and refresh totals + chart. */
  selectYear(year: string): void {
    this.selectedYear.set(year);
    const stats = this.monthlyByYear.get(year);
    this.yearIncome.set(stats ? sum(stats.income) : 0);
    this.yearCost.set(stats ? sum(stats.cost) : 0);
    this.renderChart(year);
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

  formatCurrency(value: number): string {
    return new Intl.NumberFormat("it-IT", {
      style: "currency",
      currency: "EUR",
      maximumFractionDigits: 0,
    }).format(value);
  }

  /** Build the per-year, per-month income/cost aggregation. */
  private aggregateByMonth(invoices: InvoiceSummary[]): void {
    this.monthlyByYear.clear();
    for (const inv of invoices) {
      if (!inv.data || inv.data.length < 7) continue;
      const year = inv.data.substring(0, 4);
      const month = Number(inv.data.substring(5, 7)) - 1; // 0..11
      if (Number.isNaN(month) || month < 0 || month > 11) continue;

      let stats = this.monthlyByYear.get(year);
      if (!stats) {
        stats = {
          income: new Array(12).fill(0),
          cost: new Array(12).fill(0),
          counts: new Array(12).fill(0),
        };
        this.monthlyByYear.set(year, stats);
      }
      const amount = inv.importoTotale ?? 0;
      // `purchase` = cost (owner is cessionario); everything else counts as income.
      if (inv.direction === "purchase") {
        stats.cost[month] += amount;
      } else {
        stats.income[month] += amount;
      }
      stats.counts[month] += 1;
    }
  }

  /** Render (or re-render) the monthly income/cost chart for the given year. */
  private renderChart(year: string): void {
    const canvas = this.chartCanvas?.nativeElement;
    const stats = this.monthlyByYear.get(year);
    if (!canvas || !stats) {
      return;
    }

    const currency = new Intl.NumberFormat("it-IT", {
      style: "currency",
      currency: "EUR",
      maximumFractionDigits: 0,
    });

    this.chart?.destroy();
    this.chart = new Chart(canvas, {
      type: "bar",
      data: {
        labels: MONTH_LABELS,
        datasets: [
          {
            label: "Guadagni",
            data: stats.income,
            backgroundColor: "rgba(59, 130, 246, 0.7)",
            borderColor: "rgba(59, 130, 246, 1)",
            borderWidth: 1,
            borderRadius: 6,
          },
          {
            label: "Costi",
            // Shown below the axis as negative values.
            data: stats.cost.map((v) => -v),
            backgroundColor: "rgba(239, 68, 68, 0.7)",
            borderColor: "rgba(239, 68, 68, 1)",
            borderWidth: 1,
            borderRadius: 6,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        onClick: (_event, elements) => {
          if (elements.length > 0) {
            this.viewMonthInvoices(elements[0].index);
          }
        },
        onHover: (event, elements) => {
          const target = event.native?.target as HTMLElement | undefined;
          if (target) {
            target.style.cursor = elements.length > 0 ? "pointer" : "default";
          }
        },
        plugins: {
          legend: { display: true },
          tooltip: {
            callbacks: {
              label: (ctx) => `${ctx.dataset.label}: ${currency.format(Math.abs(Number(ctx.parsed.y)))}`,
            },
          },
        },
        scales: {
          y: {
            beginAtZero: true,
            ticks: { callback: (value) => currency.format(Number(value)) },
            title: { display: true, text: "Valore (€)" },
          },
          x: {
            title: { display: true, text: "Mese" },
          },
        },
      },
    });
  }
}

/** Sum a numeric array. */
function sum(values: number[]): number {
  return values.reduce((a, b) => a + b, 0);
}
