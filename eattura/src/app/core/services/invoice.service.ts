import { Injectable } from "@angular/core";
import { TauriService } from "./tauri.service";
import type {
  InvoiceSummary,
  InvoiceDetail,
  InvoiceStatus,
  CreateInvoiceRequest,
  NotificationSummary,
  DirImportSummary,
} from "../models/invoice.model";

/**
 * Service for invoice operations via Tauri IPC.
 * All methods call Rust backend commands defined in commands/invoices.rs.
 */
@Injectable({ providedIn: "root" })
export class InvoiceService {
  constructor(private tauri: TauriService) {}

  listInvoices(): Promise<InvoiceSummary[]> {
    return this.tauri.invoke("list_invoices");
  }

  getInvoice(id: string): Promise<InvoiceDetail> {
    return this.tauri.invoke("get_invoice", { id });
  }

  createInvoice(invoice: CreateInvoiceRequest): Promise<InvoiceDetail> {
    return this.tauri.invoke("create_invoice", { invoice });
  }

  updateInvoice(
    id: string,
    invoice: CreateInvoiceRequest
  ): Promise<InvoiceDetail> {
    return this.tauri.invoke("update_invoice", { id, invoice });
  }

  deleteInvoice(id: string): Promise<void> {
    return this.tauri.invoke("delete_invoice", { id });
  }

  setStatus(id: string, status: InvoiceStatus): Promise<InvoiceDetail> {
    return this.tauri.invoke("set_invoice_status", { id, status });
  }

  nextInvoiceNumber(cedenteId: string, year: string): Promise<string> {
    return this.tauri.invoke("next_invoice_number", { cedenteId, year });
  }

  // --- PEC / SDI ---

  setPecPassword(email: string, password: string): Promise<void> {
    return this.tauri.invoke("set_pec_password", { email, password });
  }

  sendInvoicePec(id: string): Promise<string> {
    return this.tauri.invoke("send_invoice_pec", { id });
  }

  syncPec(): Promise<NotificationSummary[]> {
    return this.tauri.invoke("sync_pec");
  }

  validateInvoice(
    id: string
  ): Promise<{ valid: boolean; errors: { code: string; message: string }[] }> {
    return this.tauri.invoke("validate_invoice", { id });
  }

  exportXml(id: string): Promise<string> {
    return this.tauri.invoke("export_invoice_xml", { id });
  }

  importXml(xmlContent: string): Promise<InvoiceDetail> {
    return this.tauri.invoke("import_invoice_xml", { xmlContent });
  }

  importFromDir(dirPath: string): Promise<DirImportSummary> {
    return this.tauri.invoke("import_invoices_from_dir", { dirPath });
  }
}
