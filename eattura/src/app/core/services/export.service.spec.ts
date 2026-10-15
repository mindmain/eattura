import { describe, it, expect, beforeEach, vi } from "vitest";
import { TestBed } from "@angular/core/testing";
import { ExportService } from "./export.service";
import { TauriService } from "./tauri.service";
import { FileDownloadService } from "./file-download.service";
import type { ExportFilter } from "../models/export.model";

describe("ExportService", () => {
  let service: ExportService;
  let tauriMock: { invoke: ReturnType<typeof vi.fn> };
  let fileDownloaderMock: {
    downloadBinaryFile: ReturnType<typeof vi.fn>;
    downloadTextFile: ReturnType<typeof vi.fn>;
  };

  const mockFilter: ExportFilter = {
    year: 2026,
    period: "all",
    direction: "all",
    includePdf: true,
    includeSdiReceipts: true,
  };

  beforeEach(() => {
    tauriMock = {
      invoke: vi.fn().mockImplementation((cmd: string) => {
        if (cmd === "export_accountant_package") {
          return Promise.resolve({
            filename: "export.zip",
            zipBase64: "UEsDBA==",
            invoiceCount: 5,
            salesCount: 3,
            purchaseCount: 2,
            totalAmount: 1500,
          });
        }
        if (cmd === "export_accountant_csv") {
          return Promise.resolve({
            filename: "report.csv",
            csvContent: "id;numero;data\n1;01;2026-01-01",
            rowCount: 1,
          });
        }
        if (cmd === "export_agid_preservation_package") {
          return Promise.resolve({
            filename: "agid.zip",
            zipBase64: "UEsDBA==",
            invoiceCount: 5,
            salesCount: 3,
            purchaseCount: 2,
            totalAmount: 1500,
          });
        }
        if (cmd === "verify_agid_preservation_package") {
          return Promise.resolve({
            valid: true,
            packageId: "urn:uuid:mock",
            createdAt: "2026-10-11T10:00:00Z",
            producer: "Test",
            algorithm: "SHA-256",
            totalFiles: 10,
            verifiedFiles: 10,
            errors: [],
          });
        }
        return Promise.resolve({});
      }),
    };

    fileDownloaderMock = {
      downloadBinaryFile: vi.fn(),
      downloadTextFile: vi.fn(),
    };

    TestBed.configureTestingModule({
      providers: [
        ExportService,
        { provide: TauriService, useValue: tauriMock },
        { provide: FileDownloadService, useValue: fileDownloaderMock },
      ],
    });

    service = TestBed.inject(ExportService);
  });

  it("exportAccountantPackage invoca export_accountant_package via Tauri", async () => {
    const result = await service.exportAccountantPackage(mockFilter);
    expect(tauriMock.invoke).toHaveBeenCalledWith("export_accountant_package", { filter: mockFilter });
    expect(result.invoiceCount).toBe(5);
    expect(result.filename).toBe("export.zip");
  });

  it("exportAccountantCsv invoca export_accountant_csv via Tauri", async () => {
    const result = await service.exportAccountantCsv(mockFilter);
    expect(tauriMock.invoke).toHaveBeenCalledWith("export_accountant_csv", { filter: mockFilter });
    expect(result.rowCount).toBe(1);
  });

  it("verifyAgidPreservationPackage invoca verify_agid_preservation_package via Tauri", async () => {
    const result = await service.verifyAgidPreservationPackage("UEsDBA==");
    expect(tauriMock.invoke).toHaveBeenCalledWith("verify_agid_preservation_package", {
      zipBase64: "UEsDBA==",
    });
    expect(result.valid).toBe(true);
    expect(result.verifiedFiles).toBe(10);
  });

  it("downloadBase64 delega al FileDownloadService", () => {
    service.downloadBase64("UEsDBA==", "test.zip");
    expect(fileDownloaderMock.downloadBinaryFile).toHaveBeenCalled();
  });

  it("downloadText delega al FileDownloadService", () => {
    service.downloadText("a;b;c", "test.csv");
    expect(fileDownloaderMock.downloadTextFile).toHaveBeenCalledWith("a;b;c", "test.csv", "text/csv;charset=utf-8;");
  });
});
