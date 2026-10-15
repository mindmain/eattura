import { Injectable, inject } from "@angular/core";
import { TauriService } from "./tauri.service";
import { FileDownloadService } from "./file-download.service";
import type {
  ExportFilter,
  ExportResultDto,
  CsvExportResultDto,
  AgidPreservationVerificationResult,
} from "../models/export.model";

@Injectable({ providedIn: "root" })
export class ExportService {
  private tauri = inject(TauriService);
  private fileDownloader = inject(FileDownloadService);

  /**
   * Export the complete accountant package containing XML, courtesy PDF, SDI receipts, and Prima Nota CSV.
   */
  exportAccountantPackage(filter: ExportFilter): Promise<ExportResultDto> {
    return this.tauri.invoke<ExportResultDto>("export_accountant_package", { filter });
  }

  /**
   * Export only the Prima Nota CSV formatted with UTF-8 BOM for direct import in Zucchetti, TeamSystem, Datev, and Excel.
   */
  exportAccountantCsv(filter: ExportFilter): Promise<CsvExportResultDto> {
    return this.tauri.invoke<CsvExportResultDto>("export_accountant_csv", { filter });
  }

  /**
   * Generate an official AgID Submission Information Package (SIP / SInCRO UNI 11386:2010) with indice_conservazione.xml and SHA-256 digests.
   */
  exportAgidPreservationPackage(filter: ExportFilter): Promise<ExportResultDto> {
    return this.tauri.invoke<ExportResultDto>("export_agid_preservation_package", { filter });
  }

  /**
   * Inspect and verify the cryptographic integrity and manifest compliance of an AgID preservation ZIP package.
   */
  verifyAgidPreservationPackage(zipBase64: string): Promise<AgidPreservationVerificationResult> {
    return this.tauri.invoke<AgidPreservationVerificationResult>("verify_agid_preservation_package", {
      zipBase64,
    });
  }

  /**
   * Trigger direct browser / desktop download of a base64-encoded archive.
   */
  downloadBase64(base64Data: string, filename: string, mimeType = "application/zip"): void {
    const byteCharacters = atob(base64Data);
    const byteNumbers = new Array(byteCharacters.length);
    for (let i = 0; i < byteCharacters.length; i++) {
      byteNumbers[i] = byteCharacters.charCodeAt(i);
    }
    const byteArray = new Uint8Array(byteNumbers);
    this.fileDownloader.downloadBinaryFile(byteArray, filename, mimeType);
  }

  /**
   * Trigger download of a raw text / CSV file.
   */
  downloadText(content: string, filename: string, mimeType = "text/csv;charset=utf-8;"): void {
    this.fileDownloader.downloadTextFile(content, filename, mimeType);
  }
}
