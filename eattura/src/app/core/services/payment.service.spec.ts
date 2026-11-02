import { describe, it, expect, beforeEach, vi } from "vitest";
import { TestBed } from "@angular/core/testing";
import { PaymentService } from "./payment.service";
import { TauriService } from "./tauri.service";
import type { PaymentScheduleItem, RegisterPaymentRequest, IvaLiquidationRequest } from "../models/payment.model";

describe("PaymentService", () => {
  let service: PaymentService;
  let tauriMock: { invoke: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    tauriMock = {
      invoke: vi.fn(),
    };

    TestBed.configureTestingModule({
      providers: [
        PaymentService,
        { provide: TauriService, useValue: tauriMock },
      ],
    });
    service = TestBed.inject(PaymentService);
  });

  it("should be created", () => {
    expect(service).toBeDefined();
  });

  it("getPaymentSchedule invoca get_payment_schedule via Tauri", async () => {
    const mockItems: PaymentScheduleItem[] = [
      {
        invoiceId: "inv-1",
        numero: "10/2026",
        dataDocumento: "2026-01-01",
        dataScadenza: "2026-01-31",
        direction: "outbound",
        controparteNome: "Acme",
        controparteId: "client-1",
        importoTotale: 100,
        importoPagato: 0,
        saldoResiduo: 100,
        statoScadenza: "scaduta",
        giorniRitardo: 10,
        pagata: false,
      },
    ];
    tauriMock.invoke.mockResolvedValue(mockItems);

    const result = await service.getPaymentSchedule({ direction: "outbound" });
    expect(tauriMock.invoke).toHaveBeenCalledWith("get_payment_schedule", { filter: { direction: "outbound" } });
    expect(result).toEqual(mockItems);
  });

  it("registerPayment invoca register_invoice_payment via Tauri", async () => {
    const req: RegisterPaymentRequest = {
      invoiceId: "inv-1",
      importo: 100,
      dataPagamento: "2026-01-31",
      saldaTutto: true,
    };
    tauriMock.invoke.mockResolvedValue({ invoiceId: "inv-1", pagata: true });

    const result = await service.registerPayment(req);
    expect(tauriMock.invoke).toHaveBeenCalledWith("register_invoice_payment", { request: req });
    expect(result.pagata).toBe(true);
  });

  it("calculateIvaLiquidation invoca calculate_iva_liquidation via Tauri", async () => {
    const req: IvaLiquidationRequest = {
      year: 2026,
      period: "Q1",
      isTrimestrale: true,
      creditoPrecedente: 0,
    };
    tauriMock.invoke.mockResolvedValue({ totaleDaVersareF24: 500 });

    const result = await service.calculateIvaLiquidation(req);
    expect(tauriMock.invoke).toHaveBeenCalledWith("calculate_iva_liquidation", { request: req });
    expect(result.totaleDaVersareF24).toBe(500);
  });
});
