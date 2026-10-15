import { Injectable, inject } from "@angular/core";
import { TauriService } from "./tauri.service";
import type {
  InvoiceSummary,
  InvoiceDetail,
  InvoiceStatus,
  CreateInvoiceRequest,
  NotificationSummary,
  DirImportSummary,
  SignerInfo,
} from "../models/invoice.model";

/**
 * Service for invoice operations via Tauri IPC.
 * All methods call Rust backend commands defined in commands/invoices.rs.
 */
@Injectable({ providedIn: "root" })
export class InvoiceService {
  private tauri = inject(TauriService);

  listInvoices(): Promise<InvoiceSummary[]> {
    return this.tauri.invoke("list_invoices");
  }

  getInvoice(id: string): Promise<InvoiceDetail> {
    return this.tauri.invoke("get_invoice", { id });
  }

  createInvoice(request: CreateInvoiceRequest): Promise<InvoiceDetail> {
    return this.tauri.invoke("create_invoice", { request });
  }

  updateInvoice(id: string, request: CreateInvoiceRequest): Promise<InvoiceDetail> {
    return this.tauri.invoke("update_invoice", { id, request });
  }

  setStatus(id: string, stato: InvoiceStatus): Promise<InvoiceDetail> {
    return this.tauri.invoke("set_status", { id, stato });
  }

  generateXml(id: string): Promise<string> {
    return this.tauri.invoke("generate_xml", { id });
  }

  exportXml(id: string): Promise<string> {
    return this.tauri.invoke("export_invoice_xml", { id });
  }

  renderPdf(id: string): Promise<number[]> {
    return this.tauri.invoke("render_invoice_pdf", { id });
  }

  importXml(xmlContent: string): Promise<InvoiceDetail> {
    return this.tauri.invoke("import_xml", { xmlContent });
  }

  validateXml(xmlContent: string): Promise<{ valid: boolean; errors: string[] }> {
    return this.tauri.invoke("validate_xml", { xmlContent });
  }

  validateInvoice(
    id: string
  ): Promise<{ valid: boolean; errors: { code: string; message: string }[] }> {
    return this.tauri.invoke("validate_invoice", { id });
  }

  nextInvoiceNumber(cedenteId: string, year: string): Promise<string> {
    return this.tauri.invoke("next_invoice_number", { cedenteId, year });
  }

  deleteInvoice(id: string): Promise<void> {
    return this.tauri.invoke("delete_invoice", { id });
  }

  listNotifications(invoiceId: string): Promise<NotificationSummary[]> {
    return this.tauri.invoke("list_notifications", { invoiceId });
  }

  importFromDir(
    dirPath: string,
    direction: "sale" | "purchase" = "sale"
  ): Promise<DirImportSummary> {
    return this.tauri.invoke("import_invoices_from_dir", { dirPath, direction });
  }

  getInvoicePdf(id: string): Promise<string> {
    return this.tauri.invoke("get_invoice_pdf", { id });
  }

  verifyInvoiceSignature(id: string): Promise<SignerInfo> {
    return this.tauri.invoke("verify_invoice_signature", { id });
  }

  sendInvoicePec(id: string): Promise<string> {
    return this.tauri.invoke("send_invoice_pec", { id });
  }

  syncPec(): Promise<NotificationSummary[]> {
    return this.tauri.invoke("sync_pec");
  }

  pecSyncStatus(): Promise<PecSyncStatus> {
    return this.tauri.invoke("pec_sync_status");
  }

  signP7m(id: string, signerName?: string, signerCf?: string): Promise<number[]> {
    return this.tauri.invoke("sign_invoice_p7m", { id, signerName, signerCf });
  }

  rectifyRejectedInvoice(id: string): Promise<InvoiceDetail> {
    return this.tauri.invoke("rectify_rejected_invoice", { id });
  }

  importSdiReceipt(xmlContent: string): Promise<NotificationSummary> {
    return this.tauri.invoke("import_sdi_receipt", { xmlContent });
  }

  generateAutofatturaDraft(
    purchaseInvoiceId: string,
    autofatturaType?: string
  ): Promise<CreateInvoiceRequest> {
    return this.tauri.invoke("generate_autofattura_draft", {
      purchaseInvoiceId,
      autofatturaType,
    });
  }
}

export interface PecSyncStatus {
  configured: boolean;
  lastAttemptAt: number | null;
  lastSuccessAt: number | null;
  nextSyncAt: number;
  lastError: string | null;
}
