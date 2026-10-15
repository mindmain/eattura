import { Injectable, inject } from "@angular/core";
import { TauriService } from "./tauri.service";
import type {
  PaymentScheduleItem,
  ScheduleFilter,
  RegisterPaymentRequest,
  IvaLiquidationRequest,
  IvaLiquidationResult,
} from "../models/payment.model";

@Injectable({ providedIn: "root" })
export class PaymentService {
  private tauri = inject(TauriService);

  /**
   * Retrieve the full schedule of incoming and outgoing payments with status.
   */
  async getPaymentSchedule(filter?: ScheduleFilter): Promise<PaymentScheduleItem[]> {
    return this.tauri.invoke<PaymentScheduleItem[]>("get_payment_schedule", { filter });
  }

  /**
   * Register a full or partial payment in the local sovereign sidecar ledger.
   */
  async registerPayment(request: RegisterPaymentRequest): Promise<PaymentScheduleItem> {
    return this.tauri.invoke<PaymentScheduleItem>("register_invoice_payment", { request });
  }

  /**
   * Calculate periodic VAT liquidation (Liquidazione IVA) with split payment and 100 euro threshold.
   */
  async calculateIvaLiquidation(request: IvaLiquidationRequest): Promise<IvaLiquidationResult> {
    return this.tauri.invoke<IvaLiquidationResult>("calculate_iva_liquidation", { request });
  }
}
