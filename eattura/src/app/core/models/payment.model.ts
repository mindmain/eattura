export interface PaymentScheduleItem {
  invoiceId: string;
  numero: string;
  dataDocumento: string;
  direction: "outbound" | "inbound";
  controparteNome: string;
  controparteId: string;
  importoTotale: number;
  importoPagato: number;
  saldoResiduo: number;
  pagata: boolean;
  dataScadenza: string;
  dataUltimoPagamento?: string;
  modalitaPagamento?: string;
  giorniRitardo: number;
  statoScadenza: "scaduta" | "in_scadenza" | "futura" | "saldata";
  note?: string;
}

export interface ScheduleFilter {
  direction?: "outbound" | "inbound" | "all";
  stato?: "scaduta" | "in_scadenza" | "futura" | "saldata" | "all";
}

export interface RegisterPaymentRequest {
  invoiceId: string;
  importo: number;
  dataPagamento: string;
  modalita?: string;
  note?: string;
  saldaTutto?: boolean;
}

export interface IvaLiquidationRequest {
  year: number;
  period: string; // "M01".."M12" or "Q1".."Q4"
  creditoPrecedente?: number;
  isTrimestrale?: boolean;
}

export interface IvaAliquotaDettaglio {
  aliquota: number;
  imponibileVendite: number;
  impostaVendite: number;
  imponibileAcquisti: number;
  impostaAcquisti: number;
}

export interface IvaLiquidationResult {
  year: number;
  period: string;
  periodoLabel: string;
  dataScadenzaVersamento: string;
  codiceTributoF24: string;
  imponibileVendite: number;
  ivaVenditeTotale: number;
  ivaVenditeSplitPayment: number;
  ivaVenditeEsigibile: number;
  imponibileAcquisti: number;
  ivaAcquistiDetraibile: number;
  creditoIvaPrecedente: number;
  saldoIvaPeriodo: number;
  interessiTrimestrali1Perc: number;
  totaleDaVersareF24: number;
  creditoDaRiportare: number;
  versamentoSottoSoglia100Euro: boolean;
  noteFiscali: string[];
  aliquoteDettaglio: IvaAliquotaDettaglio[];
}
