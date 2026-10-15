export interface ExportFilter {
  year: number;
  period: string; // 'all' | 'q1' | 'q2' | 'q3' | 'q4' | '01'..'12'
  direction: string; // 'all' | 'sale' | 'purchase'
  includePdf: boolean;
  includeSdiReceipts: boolean;
}

export interface ExportResultDto {
  filename: string;
  zipBase64: string;
  invoiceCount: number;
  salesCount: number;
  purchaseCount: number;
  totalAmount: number;
}

export interface CsvExportResultDto {
  filename: string;
  csvContent: string;
  rowCount: number;
}

export interface AgidPreservationVerificationResult {
  valid: boolean;
  packageId: string;
  createdAt: string;
  producer: string;
  algorithm: string;
  totalFiles: number;
  verifiedFiles: number;
  errors: string[];
}
