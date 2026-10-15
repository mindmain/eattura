import { describe, it, expect, beforeEach, vi } from "vitest";
import { TestBed } from "@angular/core/testing";
import { ExportComponent } from "./export.component";
import { ExportService } from "../../core/services/export.service";
import { NotificationService } from "../../core/services/notification.service";
import type { ExportResultDto, CsvExportResultDto, AgidPreservationVerificationResult } from "../../core/models/export.model";

describe("ExportComponent", () => {
  let component: ExportComponent;
  let exportServiceMock: {
    exportAccountantPackage: ReturnType<typeof vi.fn>;
    exportAccountantCsv: ReturnType<typeof vi.fn>;
    exportAgidPreservationPackage: ReturnType<typeof vi.fn>;
    verifyAgidPreservationPackage: ReturnType<typeof vi.fn>;
    downloadBase64: ReturnType<typeof vi.fn>;
    downloadText: ReturnType<typeof vi.fn>;
  };
  let notifyMock: {
    success: ReturnType<typeof vi.fn>;
    error: ReturnType<typeof vi.fn>;
  };

  const mockZipResult: ExportResultDto = {
    filename: "Export_2026.zip",
    zipBase64: "UEsDBA==",
    invoiceCount: 10,
    salesCount: 6,
    purchaseCount: 4,
    totalAmount: 5000,
  };

  const mockCsvResult: CsvExportResultDto = {
    filename: "Prima_Nota_2026.csv",
    csvContent: "id;numero;data\n1;01;2026-01-01",
    rowCount: 10,
  };

  const mockVerificationResult: AgidPreservationVerificationResult = {
    valid: true,
    packageId: "urn:uuid:1234",
    createdAt: "2026-10-11T10:00:00Z",
    producer: "Test Srl",
    algorithm: "SHA-256",
    totalFiles: 20,
    verifiedFiles: 20,
    errors: [],
  };

  beforeEach(() => {
    exportServiceMock = {
      exportAccountantPackage: vi.fn().mockResolvedValue(mockZipResult),
      exportAccountantCsv: vi.fn().mockResolvedValue(mockCsvResult),
      exportAgidPreservationPackage: vi.fn().mockResolvedValue(mockZipResult),
      verifyAgidPreservationPackage: vi.fn().mockResolvedValue(mockVerificationResult),
      downloadBase64: vi.fn(),
      downloadText: vi.fn(),
    };

    notifyMock = {
      success: vi.fn(),
      error: vi.fn(),
    };

    TestBed.configureTestingModule({
      providers: [
        { provide: ExportService, useValue: exportServiceMock },
        { provide: NotificationService, useValue: notifyMock },
      ],
    });

    component = TestBed.runInInjectionContext(() => new ExportComponent());
  });

  it("inizializza con i parametri corretti", () => {
    expect(component.selectedYear()).toBe(new Date().getFullYear());
    expect(component.selectedPeriod()).toBe("all");
    expect(component.selectedDirection()).toBe("all");
    expect(component.includePdf()).toBe(true);
    expect(component.includeSdiReceipts()).toBe(true);
  });

  it("exportAccountantZip scarica lo zip e notifica il successo", async () => {
    await component.exportAccountantZip();

    expect(exportServiceMock.exportAccountantPackage).toHaveBeenCalledWith(
      expect.objectContaining({
        year: component.selectedYear(),
        period: "all",
      })
    );
    expect(exportServiceMock.downloadBase64).toHaveBeenCalledWith(
      mockZipResult.zipBase64,
      mockZipResult.filename
    );
    expect(notifyMock.success).toHaveBeenCalled();
  });

  it("exportPrimaNotaCsv scarica il CSV e notifica il successo", async () => {
    await component.exportPrimaNotaCsv();

    expect(exportServiceMock.exportAccountantCsv).toHaveBeenCalled();
    expect(exportServiceMock.downloadText).toHaveBeenCalledWith(
      mockCsvResult.csvContent,
      mockCsvResult.filename,
      "text/csv;charset=utf-8;"
    );
    expect(notifyMock.success).toHaveBeenCalled();
  });

  it("exportAgidPackage scarica il pacchetto SIP", async () => {
    await component.exportAgidPackage();

    expect(exportServiceMock.exportAgidPreservationPackage).toHaveBeenCalled();
    expect(exportServiceMock.downloadBase64).toHaveBeenCalledWith(
      mockZipResult.zipBase64,
      mockZipResult.filename
    );
    expect(notifyMock.success).toHaveBeenCalled();
  });
});
