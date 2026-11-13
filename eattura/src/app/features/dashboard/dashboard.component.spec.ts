import { signal } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { provideRouter } from "@angular/router";
import { describe, expect, it, vi } from "vitest";
import type { InvoiceSummary } from "../../core/models/invoice.model";
import { ThemeService } from "../../core/services/theme.service";
import { ClientStore } from "../../core/stores/client.store";
import { InvoiceStore } from "../../core/stores/invoice.store";
import { DashboardComponent } from "./dashboard.component";

function invoice(stato: InvoiceSummary["stato"], importoTotale: number, overrides: Partial<InvoiceSummary> = {}): InvoiceSummary {
  return {
    id: stato,
    numero: "1",
    data: `${new Date().getFullYear()}-03-01`,
    tipoDocumento: "TD01",
    importoTotale,
    cedenteDenominazione: "Seller",
    cessionarioDenominazione: "Buyer",
    direction: "sale",
    stato,
    ...overrides,
  };
}

function setup(records: InvoiceSummary[]) {
  const invoices = signal(records);
  TestBed.configureTestingModule({
    providers: [
      provideRouter([]),
      { provide: InvoiceStore, useValue: { invoices, loadAll: vi.fn().mockResolvedValue(undefined) } },
      { provide: ClientStore, useValue: { loadAll: vi.fn().mockResolvedValue(undefined) } },
      { provide: ThemeService, useValue: { effectiveTheme: signal("light") } },
    ],
  });
  const component = TestBed.runInInjectionContext(() => new DashboardComponent());
  component.selectYear(component.annualYears[7]);
  return { component, invoices };
}

describe("Dashboard invoice totals", () => {
  it("includes accepted and imported sales in revenue and reports pending sales separately", () => {
    const { component } = setup([
      invoice("accepted", 100),
      invoice("accepted", 20, { tipoDocumento: "TD04" }),
      invoice("sent", 50),
      invoice("sent", 10, { tipoDocumento: "TD08" }),
      invoice("validated", 200),
      invoice("imported", 300),
      invoice("draft", 400),
      invoice("rejected", 500),
      invoice("imported", 30, { direction: "purchase" }),
    ]);

    const march = 2;
    const monthly = component.monthlyByYear().get(component.selectedYear());
    expect(monthly?.income[march]).toBe(380);
    expect(monthly?.pendingIncome[march]).toBe(40);
    expect(component.yearIncome()).toBe(380);
    expect(component.yearPendingIncome()).toBe(40);
    expect(component.yearCost()).toBe(30);
    expect(component.annualIncome().at(-1)).toBe(380);
  });

  it("updates both totals when sent invoices receive an SdI outcome", () => {
    const accepted = invoice("sent", 100, { id: "to-accept" });
    const rejected = invoice("sent", 50, { id: "to-reject" });
    const { component, invoices } = setup([accepted, rejected]);

    expect(component.yearIncome()).toBe(0);
    expect(component.yearPendingIncome()).toBe(150);

    const confirmed = { ...accepted, stato: "accepted" } as const;
    invoices.set([confirmed, rejected]);

    expect(component.yearIncome()).toBe(100);
    expect(component.yearPendingIncome()).toBe(50);

    invoices.set([confirmed, { ...rejected, stato: "rejected" }]);

    expect(component.yearIncome()).toBe(100);
    expect(component.yearPendingIncome()).toBe(0);
  });
});
