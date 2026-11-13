import { Injector } from "@angular/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DriveService } from "./drive.service";
import { TauriService } from "./tauri.service";
import { InvoiceStore } from "../stores/invoice.store";
import { ClientStore } from "../stores/client.store";

vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true }));

describe("DriveService", () => {
  const invoke = vi.fn();
  const refreshInvoices = vi.fn();
  const refreshClients = vi.fn();
  let drive: DriveService;

  beforeEach(() => {
    vi.resetAllMocks();
    drive = Injector.create({ providers: [
      DriveService,
      { provide: TauriService, useValue: { invoke } },
      { provide: InvoiceStore, useValue: { refresh: refreshInvoices } },
      { provide: ClientStore, useValue: { refresh: refreshClients } },
    ] }).get(DriveService);
  });

  it("imports at startup and refreshes the local invoice and client views", async () => {
    invoke.mockResolvedValueOnce({ folderName: "Shared invoices" }).mockResolvedValueOnce({
      imported: 1, unchanged: 2, ignored: 0, failed: 0, errors: [],
    });

    await drive.start();

    expect(invoke.mock.calls).toEqual([["get_drive_status"], ["sync_drive"]]);
    expect(drive.folderName()).toBe("Shared invoices");
    expect(drive.lastSync()?.imported).toBe(1);
    expect(refreshInvoices).toHaveBeenCalledOnce();
    expect(refreshClients).toHaveBeenCalledOnce();
  });

  it("skips the startup import when the user has no connected folder", async () => {
    invoke.mockResolvedValueOnce({ folderName: null });

    await drive.start();

    expect(invoke.mock.calls).toEqual([["get_drive_status"]]);
    expect(drive.lastSync()).toBeNull();
  });
});
