import { Injector } from "@angular/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { listen } from "@tauri-apps/api/event";
import { DriveService, type DriveSyncReport } from "./drive.service";
import { TauriService } from "./tauri.service";
import { InvoiceStore } from "../stores/invoice.store";
import { ClientStore } from "../stores/client.store";

vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

describe("DriveService", () => {
  const invoke = vi.fn();
  const refreshInvoices = vi.fn();
  const refreshClients = vi.fn();
  let eventHandler: ((event: { payload: DriveSyncReport }) => void) | undefined;
  let drive: DriveService;

  beforeEach(() => {
    vi.resetAllMocks();
    eventHandler = undefined;
    vi.mocked(listen).mockImplementation(async (_event, handler) => {
      eventHandler = handler as (event: { payload: DriveSyncReport }) => void;
      return () => undefined;
    });
    drive = Injector.create({
      providers: [
        DriveService,
        { provide: TauriService, useValue: { invoke } },
        { provide: InvoiceStore, useValue: { refresh: refreshInvoices } },
        { provide: ClientStore, useValue: { refresh: refreshClients } },
      ],
    }).get(DriveService);
  });

  it("loads backup status on startup and refreshes stores when a remote backup is downloaded", async () => {
    invoke
      .mockResolvedValueOnce({
        connected: true,
        accountEmail: "utente@example.it",
        backupName: "Eattura / eattura-backup.zip",
        lastBackupAt: "2026-11-25T11:00:00Z",
        lastCheckedAt: 1764068400000,
        lastAction: "up_to_date",
      })
      .mockResolvedValueOnce({
        connected: true,
        accountEmail: "utente@example.it",
        backupName: "Eattura / eattura-backup.zip",
        lastBackupAt: "2026-11-25T11:05:00Z",
        lastCheckedAt: 1764068700000,
        lastAction: "downloaded",
      });

    await drive.start();
    expect(drive.status().connected).toBe(true);
    expect(drive.status().accountEmail).toBe("utente@example.it");

    eventHandler?.({
      payload: {
        action: "downloaded",
        modifiedTime: "2026-11-25T11:05:00Z",
        invoices: 12,
        clients: 4,
        notifications: 8,
      },
    });
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(drive.lastSync()?.action).toBe("downloaded");
    expect(refreshInvoices).toHaveBeenCalledOnce();
    expect(refreshClients).toHaveBeenCalledOnce();
  });

  it("runs manual syncNow and skips store reload when backup was only uploaded", async () => {
    invoke
      .mockResolvedValueOnce({
        action: "uploaded",
        modifiedTime: "2026-11-25T11:10:00Z",
        invoices: 5,
        clients: 2,
        notifications: 3,
      })
      .mockResolvedValueOnce({
        connected: true,
        accountEmail: "utente@example.it",
        backupName: "Eattura / eattura-backup.zip",
        lastBackupAt: "2026-11-25T11:10:00Z",
        lastCheckedAt: 1764069000000,
        lastAction: "uploaded",
      });

    await drive.syncNow();
    expect(invoke.mock.calls).toEqual([["sync_drive"], ["get_drive_status"]]);
    expect(drive.lastSync()?.action).toBe("uploaded");
    expect(refreshInvoices).not.toHaveBeenCalled();
  });
});
