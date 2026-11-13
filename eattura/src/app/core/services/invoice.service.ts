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

  createInvoice(invoice: CreateInvoiceRequest): Promise<InvoiceDetail> {
    return this.tauri.invoke("create_invoice", { invoice });
  }

  updateInvoice(id: string, invoice: CreateInvoiceRequest, expectedRevision?: string | null): Promise<InvoiceDetail> {
    return this.tauri.invoke("update_invoice", { id, invoice, expectedRevision });
  }

  setStatus(id: string, status: InvoiceStatus): Promise<InvoiceDetail> {
    return this.tauri.invoke("set_invoice_status", { id, status });
  }

  exportXml(id: string): Promise<string> {
    return this.tauri.invoke("export_invoice_xml", { id });
  }

  renderPdf(id: string): Promise<number[]> {
    return this.tauri.invoke("render_invoice_pdf", { id });
  }

  importXml(xmlContent: string): Promise<InvoiceDetail> {
    return this.tauri.invoke("import_invoice_xml", { xmlContent });
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

  importFromDir(dirPath: string): Promise<DirImportSummary> {
    return this.tauri.invoke("import_invoices_from_dir", { dirPath });
  }

  verifyP7mSignature(p7mBytes: number[]): Promise<SignerInfo> {
    return this.tauri.invoke("verify_p7m_signature", { p7mBytes });
  }

  sendInvoicePec(id: string): Promise<string> {
    return this.tauri.invoke("send_invoice_pec", { id });
  }

  resolvePecAttempt(id: string, filename: string, sent: boolean): Promise<void> {
    return this.tauri.invoke("resolve_pec_attempt", { id, filename, sent });
  }

  syncPec(): Promise<NotificationSummary[]> {
    return this.tauri.invoke("sync_pec");
  }

  pecSyncStatus(): Promise<PecSyncStatus> {
    return this.tauri.invoke("pec_sync_status");
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
  inProgress?: boolean;
  configured: boolean;
  lastAttemptAt: number | null;
  lastSuccessAt: number | null;
  nextSyncAt: number;
  lastError: string | null;
  sdiAddress?: string;
  sdiAddressDetected?: string | null;
  sdiAddressVerified?: boolean;
}
