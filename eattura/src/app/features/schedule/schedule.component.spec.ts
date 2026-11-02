import { describe, it, expect, beforeEach, vi } from "vitest";
import { TestBed } from "@angular/core/testing";
import { FormBuilder } from "@angular/forms";
import { ScheduleComponent } from "./schedule.component";
import { PaymentService } from "../../core/services/payment.service";
import { NotificationService } from "../../core/services/notification.service";
import type { PaymentScheduleItem, IvaLiquidationResult } from "../../core/models/payment.model";

describe("ScheduleComponent", () => {
  let component: ScheduleComponent;
  let paymentServiceMock: {
    getPaymentSchedule: ReturnType<typeof vi.fn>;
    registerPayment: ReturnType<typeof vi.fn>;
    calculateIvaLiquidation: ReturnType<typeof vi.fn>;
  };
  let notifyMock: {
    success: ReturnType<typeof vi.fn>;
    error: ReturnType<typeof vi.fn>;
  };

  const mockScheduleItems: PaymentScheduleItem[] = [
    {
      invoiceId: "inv-1",
      numero: "01/2026",
      dataDocumento: "2026-01-10",
      dataScadenza: "2026-01-31",
      direction: "outbound",
      controparteNome: "Cliente Alfa",
      controparteId: "client-1",
      importoTotale: 1220,
      importoPagato: 0,
      saldoResiduo: 1220,
      statoScadenza: "scaduta",
      giorniRitardo: 25,
      pagata: false,
    },
    {
      invoiceId: "inv-2",
      numero: "02/2026",
      dataDocumento: "2026-02-15",
      dataScadenza: "2026-02-28",
      direction: "inbound",
      controparteNome: "Fornitore Beta",
      controparteId: "client-2",
      importoTotale: 500,
      importoPagato: 0,
      saldoResiduo: 500,
      statoScadenza: "in_scadenza",
      giorniRitardo: 0,
      pagata: false,
    },
    {
      invoiceId: "inv-3",
      numero: "03/2026",
      dataDocumento: "2026-03-01",
      dataScadenza: "2026-03-31",
      direction: "outbound",
      controparteNome: "Cliente Gamma",
      controparteId: "client-3",
      importoTotale: 300,
      importoPagato: 300,
      saldoResiduo: 0,
      statoScadenza: "saldata",
      giorniRitardo: 0,
      pagata: true,
    },
  ];

  const mockIvaResult: IvaLiquidationResult = {
    year: 2026,
    period: "Q1",
    periodoLabel: "Q1 2026",
    dataScadenzaVersamento: "2026-05-16",
    imponibileVendite: 1000,
    ivaVenditeTotale: 220,
    ivaVenditeSplitPayment: 0,
    ivaVenditeEsigibile: 220,
    imponibileAcquisti: 500,
    ivaAcquistiDetraibile: 110,
    creditoIvaPrecedente: 0,
    saldoIvaPeriodo: 110,
    interessiTrimestrali1Perc: 1.1,
    totaleDaVersareF24: 111.1,
    versamentoSottoSoglia100Euro: false,
    codiceTributoF24: "6031",
    creditoDaRiportare: 0,
    aliquoteDettaglio: [],
    noteFiscali: [],
  };

  beforeEach(async () => {
    paymentServiceMock = {
      getPaymentSchedule: vi.fn().mockResolvedValue(mockScheduleItems),
      registerPayment: vi.fn().mockResolvedValue(mockScheduleItems[0]),
      calculateIvaLiquidation: vi.fn().mockResolvedValue(mockIvaResult),
    };

    notifyMock = {
      success: vi.fn(),
      error: vi.fn(),
    };

    TestBed.configureTestingModule({
      providers: [
        FormBuilder,
        { provide: PaymentService, useValue: paymentServiceMock },
        { provide: NotificationService, useValue: notifyMock },
      ],
    });

    component = TestBed.runInInjectionContext(() => new ScheduleComponent());
    await component.ngOnInit();
  });

  it("carica le scadenze e calcola la liquidazione IVA all'init", () => {
    expect(paymentServiceMock.getPaymentSchedule).toHaveBeenCalled();
    expect(paymentServiceMock.calculateIvaLiquidation).toHaveBeenCalled();
    expect(component.scheduleItems()).toHaveLength(3);
    expect(component.liquidationResult()).toEqual(mockIvaResult);
  });

  it("calcola correttamente le metriche reattive KPIs", () => {
    const m = component.metrics();
    expect(m.totalScaduto).toBe(1220);
    expect(m.countScaduto).toBe(1);
    expect(m.totalInScadenza).toBe(500);
    expect(m.countInScadenza).toBe(1);
    expect(m.totalCreditiAttivi).toBe(1220); // 1220 residui + 0 da saldata
    expect(m.totalDebitiPassivi).toBe(500);
  });

  it("filtra per flusso directionFilter", () => {
    component.setDirectionFilter("outbound");
    expect(component.filteredSchedule()).toHaveLength(2);

    component.setDirectionFilter("inbound");
    expect(component.filteredSchedule()).toHaveLength(1);
    expect(component.filteredSchedule()[0].controparteNome).toBe("Fornitore Beta");
  });

  it("filtra per stato statusFilter", () => {
    component.setStatusFilter("scaduta");
    expect(component.filteredSchedule()).toHaveLength(1);
    expect(component.filteredSchedule()[0].invoiceId).toBe("inv-1");

    component.setStatusFilter("saldata");
    expect(component.filteredSchedule()).toHaveLength(1);
    expect(component.filteredSchedule()[0].invoiceId).toBe("inv-3");
  });

  it("apre e chiude la modale di pagamento", () => {
    component.openPaymentModal(mockScheduleItems[0]);
    expect(component.selectedItemForPayment()).toEqual(mockScheduleItems[0]);
    expect(component.paymentForm.get("importo")?.value).toBe(1220);

    component.closePaymentModal();
    expect(component.selectedItemForPayment()).toBeNull();
  });

  it("invia il pagamento con submitPayment e ricarica lo scadenzario", async () => {
    component.openPaymentModal(mockScheduleItems[0]);
    await component.submitPayment();

    expect(paymentServiceMock.registerPayment).toHaveBeenCalledWith(
      expect.objectContaining({
        invoiceId: "inv-1",
        importo: 1220,
        saldaTutto: true,
      })
    );
    expect(notifyMock.success).toHaveBeenCalled();
    expect(component.selectedItemForPayment()).toBeNull();
  });

  it("cambia regime di liquidazione periodico e ricalcola", () => {
    component.onPeriodTypeChange(false);
    expect(component.ivaForm.get("isTrimestrale")?.value).toBe(false);
    expect(component.ivaForm.get("period")?.value).toBe("M01");
    expect(paymentServiceMock.calculateIvaLiquidation).toHaveBeenCalledTimes(2);
  });
});
