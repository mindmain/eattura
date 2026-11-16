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
  it("includes delivered, accepted and imported sales in revenue and reports pending sales separately", () => {
    const { component } = setup([
      invoice("delivered", 75),
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
    expect(monthly?.income[march]).toBe(455);
    expect(monthly?.pendingIncome[march]).toBe(40);
    expect(component.yearIncome()).toBe(455);
    expect(component.yearPendingIncome()).toBe(40);
    expect(component.yearCost()).toBe(30);
    expect(component.annualIncome().at(-1)).toBe(455);
    expect(component.annualPendingIncome().at(-1)).toBe(40);
    expect(component.confirmedInvoicesCount()).toBe(5);
    expect(component.sentCount()).toBe(2);
    expect(component.draftCount()).toBe(1);
  });

  it("updates both totals when sent invoices receive an SdI outcome", () => {
    const delivered = invoice("sent", 100, { id: "to-deliver" });
    const rejected = invoice("sent", 50, { id: "to-reject" });
    const { component, invoices } = setup([delivered, rejected]);

    expect(component.yearIncome()).toBe(0);
    expect(component.yearPendingIncome()).toBe(150);
    expect(component.confirmedInvoicesCount()).toBe(0);
    expect(component.sentCount()).toBe(2);

    const confirmed = { ...delivered, stato: "delivered" } as const;
    invoices.set([confirmed, rejected]);

    expect(component.yearIncome()).toBe(100);
    expect(component.yearPendingIncome()).toBe(50);
    expect(component.confirmedInvoicesCount()).toBe(1);
    expect(component.sentCount()).toBe(1);

    invoices.set([confirmed, { ...rejected, stato: "rejected" }]);

    expect(component.yearIncome()).toBe(100);
    expect(component.yearPendingIncome()).toBe(0);
    expect(component.confirmedInvoicesCount()).toBe(1);
    expect(component.sentCount()).toBe(0);
  });

  it("keeps all sent sales (including MC non-delivery) in yellow pending totals outside total revenue", () => {
    const { component } = setup([
      invoice("delivered", 120, { id: "delivered-1" }),
      invoice("accepted", 80, { id: "accepted-1", isPa: true }),
      invoice("sent", 250, { id: "b2b-mc", hasMcNotice: true, isPa: false }),
      invoice("sent", 60, { id: "b2b-pending", hasMcNotice: false, isPa: false }),
    ]);

    expect(component.yearIncome()).toBe(200);
    expect(component.yearPendingIncome()).toBe(310);
    expect(component.annualIncome().at(-1)).toBe(200);
    expect(component.annualPendingIncome().at(-1)).toBe(310);
    expect(component.confirmedInvoicesCount()).toBe(2);
    expect(component.sentCount()).toBe(2);
  });
});
