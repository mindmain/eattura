/** Invoice direction relative to the owner company. */
export type InvoiceDirection = "sale" | "purchase" | "unknown";

export interface InvoiceSummary {
  id: string;
  numero: string;
  data: string;
  tipoDocumento: string;
  importoTotale: number | null;
  cedenteDenominazione: string;
  cessionarioDenominazione: string;
  stato: InvoiceStatus;
  /** `sale` = income (owner is cedente), `purchase` = cost (owner is cessionario). */
  direction: InvoiceDirection;
}

export interface InvoiceDetail extends InvoiceSummary {
  divisa: string;
  causale: string[];
  linee: InvoiceLine[];
  pagamenti: PaymentDetail[];
}

export interface InvoiceLine {
  numeroLinea: number;
  descrizione: string;
  quantita: number | null;
  unitaMisura: string | null;
  prezzoUnitario: number;
  prezzoTotale: number;
  aliquotaIva: number;
  natura: string | null;
}

export interface PaymentDetail {
  modalitaPagamento: string;
  importoPagamento: number;
  dataScadenzaPagamento: string | null;
  iban: string | null;
}

export type InvoiceStatus = "draft" | "validated" | "sent" | "accepted" | "rejected";

export interface DirImportSummary {
  totalFiles: number;
  imported: number;
  skipped: number;
  errors: { file: string; error: string }[];
}

export interface NotificationSummary {
  tipo: string;
  identificativoSdi: string | null;
  nomeFile: string | null;
  invoiceId: string | null;
  newStatus: string | null;
}

export interface CreateInvoiceRequest {
  numero: string;
  data: string;
  tipoDocumento: string;
  divisa: string;
  cedenteId: string;
  cessionarioId: string;
  linee: CreateLineRequest[];
  pagamenti: CreatePaymentRequest[];
}

export interface CreateLineRequest {
  descrizione: string;
  quantita: number | null;
  unitaMisura: string | null;
  prezzoUnitario: number;
  aliquotaIva: number;
  natura: string | null;
}

export interface CreatePaymentRequest {
  modalitaPagamento: string;
  importoPagamento: number;
  dataScadenzaPagamento: string | null;
  iban: string | null;
}
