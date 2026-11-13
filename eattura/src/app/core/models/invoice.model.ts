/** Invoice direction relative to the owner company. */
export type InvoiceDirection = "sale" | "purchase" | "unknown";

export interface SignerInfo {
  isSigned: boolean;
  signerName?: string | null;
  signerFiscalCode?: string | null;
  issuer?: string | null;
  signingTime?: string | null;
  algorithm?: string | null;
  digestHex?: string | null;
}

export interface DatiFatturaCollegataDetail {
  idDocumento: string;
  data?: string | null;
  numItem?: string | null;
  codiceCig?: string | null;
  codiceCup?: string | null;
}

export interface DatiDocumentoTracciabilitaDetail {
  tipoDocumentoRif?: string | null;
  idDocumento: string;
  data?: string | null;
  numItem?: string | null;
  codiceCommessaConvenzione?: string | null;
  codiceCup?: string | null;
  codiceCig?: string | null;
}

export interface DatiRitenutaDetail {
  tipoRitenuta: string;
  importoRitenuta: number;
  aliquotaRitenuta: number;
  causalePagamento: string;
}

export interface DatiBolloDetail {
  bolloVirtuale: string;
  importoBollo?: number | null;
}

export interface DatiCassaDetail {
  tipoCassa: string;
  alCassa: number;
  importoContributoCassa: number;
  imponibileCassa?: number | null;
  aliquotaIva: number;
  ritenuta?: string | null;
  natura?: string | null;
}

export interface SdiError {
  codice: string;
  descrizione: string;
}

export interface SdiNotificationDetail {
  id: string;
  tipo: string;
  identificativoSdi?: string;
  nomeFile?: string;
  esito?: string;
  dataOraRicezione?: string;
  destinatario?: string;
  note?: string;
  errori: SdiError[];
  rawXml: string;
}

export interface AttachmentDetail {
  nomeAttachment: string;
  algoritmoCompressione?: string | null;
  formatoAttachment?: string | null;
  descrizioneAttachment?: string | null;
  sizeBytes: number;
  attachmentBase64: string;
}

export interface CreateAttachmentRequest {
  nomeAttachment: string;
  algoritmoCompressione?: string | null;
  formatoAttachment?: string | null;
  descrizioneAttachment?: string | null;
  attachmentBase64: string;
}

export interface DatiDdtDetail {
  numeroDdt: string;
  dataDdt: string;
  riferimentoNumeroLinea?: number[];
}

export interface InvoiceSummary {
  id: string;
  numero: string;
  data: string;
  tipoDocumento: string;
  importoTotale: number | null;
  totalCalculated?: boolean;
  cedenteDenominazione: string;
  cessionarioDenominazione: string;
  stato: InvoiceStatus;
  taxIdErrors?: { code: string; message: string; elementPath: string | null }[];
  /** `sale` = income (owner is cedente), `purchase` = cost (owner is cessionario). */
  direction: InvoiceDirection;
  /** Searchable metadata from the in-memory desktop index. */
  searchText?: string;
  isSigned?: boolean;
  isAutofattura?: boolean;
  isPa?: boolean;
  isSplitPayment?: boolean;
}

export interface InvoiceDetail extends InvoiceSummary {
  revision?: string | null;
  divisa: string;
  cedenteId: string;
  cessionarioId: string;
  causale: string[];
  linee: InvoiceLine[];
  pagamenti: PaymentDetail[];
  signerInfo?: SignerInfo | null;
  notifications?: SdiNotificationDetail[];
  nettoAPagare?: number | null;
  datiRitenuta?: DatiRitenutaDetail[];
  datiBollo?: DatiBolloDetail | null;
  datiCassaPrevidenziale?: DatiCassaDetail[];
  isAutofattura?: boolean;
  soggettoEmittente?: string | null;
  datiFattureCollegate?: DatiFatturaCollegataDetail[];
  isPa?: boolean;
  isSplitPayment?: boolean;
  esigibilitaIva?: string | null;
  formatoTrasmissione?: string | null;
  codiceDestinatario?: string | null;
  pecDestinatario?: string | null;
  pendingPecFilename?: string | null;
  activePecFilename?: string | null;
  recoverablePecFilename?: string | null;
  datiOrdineAcquisto?: DatiDocumentoTracciabilitaDetail[];
  datiContratto?: DatiDocumentoTracciabilitaDetail[];
  datiConvenzione?: DatiDocumentoTracciabilitaDetail[];
  datiDdt?: DatiDdtDetail[];
  allegati?: AttachmentDetail[];
}

export interface InvoiceLineFields {
  tipoCessionePrestazione?: "SC" | "PR" | "AB" | "AC" | null;
  codiceArticolo?: { codiceTipo: string; codiceValore: string }[];
  scontoMaggiorazione?: { tipo: "SC" | "MG"; percentuale?: number | null; importo?: number | null }[];
  ritenuta?: "SI" | null;
  riferimentoAmministrazione?: string | null;
  altriDatiGestionali?: {
    tipoDato: string;
    riferimentoTesto?: string | null;
    riferimentoNumero?: number | null;
    riferimentoData?: string | null;
  }[];
}

export interface InvoiceLine extends InvoiceLineFields {
  numeroLinea: number;
  descrizione: string;
  quantita: number | null;
  unitaMisura: string | null;
  dataInizioPeriodo?: string | null;
  dataFinePeriodo?: string | null;
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

export type InvoiceStatus = "draft" | "validated" | "sent" | "accepted" | "rejected" | "imported";

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
  causale?: string[];
  linee: CreateLineRequest[];
  pagamenti: CreatePaymentRequest[];
  datiRitenuta?: DatiRitenutaDetail[] | null;
  datiBollo?: DatiBolloDetail | null;
  datiCassaPrevidenziale?: DatiCassaDetail[] | null;
  isAutofattura?: boolean;
  soggettoEmittente?: string | null;
  datiFattureCollegate?: DatiFatturaCollegataDetail[] | null;
  isPa?: boolean;
  isSplitPayment?: boolean;
  esigibilitaIva?: string | null;
  datiOrdineAcquisto?: DatiDocumentoTracciabilitaDetail[] | null;
  datiContratto?: DatiDocumentoTracciabilitaDetail[] | null;
  datiConvenzione?: DatiDocumentoTracciabilitaDetail[] | null;
  datiDdt?: DatiDdtDetail[] | null;
  allegati?: CreateAttachmentRequest[] | null;
}

export interface CreateLineRequest extends InvoiceLineFields {
  descrizione: string;
  quantita: number | null;
  unitaMisura: string | null;
  dataInizioPeriodo?: string | null;
  dataFinePeriodo?: string | null;
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
