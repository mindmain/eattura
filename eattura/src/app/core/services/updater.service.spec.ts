import { TestBed } from "@angular/core/testing";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { isTauri } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { AppLogService } from "./app-log.service";
import { UpdaterService } from "./updater.service";

vi.mock("@tauri-apps/api/core", () => ({ isTauri: vi.fn() }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: vi.fn() }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: vi.fn() }));

function mockUpdate(overrides: Partial<Update> = {}): Update {
  return {
    available: true,
    currentVersion: "0.1.0",
    version: "0.2.0",
    body: "Correzioni SdI e miglioramenti PEC",
    rawJson: {},
    download: vi.fn(async (onEvent?: (event: DownloadEvent) => void) => {
      onEvent?.({ event: "Started", data: { contentLength: 200 } });
      onEvent?.({ event: "Progress", data: { chunkLength: 100 } });
      onEvent?.({ event: "Progress", data: { chunkLength: 100 } });
      onEvent?.({ event: "Finished" });
    }),
    install: vi.fn().mockResolvedValue(undefined),
    downloadAndInstall: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
    rid: 1,
    ...overrides,
  } as unknown as Update;
}

describe("UpdaterService", () => {
  let log: { record: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(isTauri).mockReturnValue(true);
    log = { record: vi.fn().mockResolvedValue(undefined) };
    TestBed.configureTestingModule({
      providers: [UpdaterService, { provide: AppLogService, useValue: log }],
    });
  });

  it("skips update checks in browser preview mode", async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    const service = TestBed.inject(UpdaterService);
    await service.start();
    await service.checkForUpdates();
    expect(check).not.toHaveBeenCalled();
    expect(service.status()).toBe("idle");
  });

  it("silently ignores offline or missing release errors during startup", async () => {
    vi.mocked(check).mockRejectedValue(new Error("Network unreachable"));
    const service = TestBed.inject(UpdaterService);
    await service.start();
    expect(service.status()).toBe("idle");
    expect(service.error()).toBeNull();
    expect(service.bannerVisible()).toBe(false);
    expect(log.record).toHaveBeenCalledWith("warn", "updater", expect.any(Error));
  });

  it("downloads verified updates on startup and restarts on user confirmation", async () => {
    const update = mockUpdate();
    vi.mocked(check).mockResolvedValue(update);
    const service = TestBed.inject(UpdaterService);

    await service.start();
    expect(service.status()).toBe("ready");
    expect(service.availableVersion()).toBe("0.2.0");
    expect(service.releaseNotes()).toBe("Correzioni SdI e miglioramenti PEC");
    expect(service.progress()).toBe(100);
    expect(service.bannerVisible()).toBe(true);

    service.dismissBanner();
    expect(service.bannerVisible()).toBe(false);

    await service.installAndRestart();
    expect(update.install).toHaveBeenCalledOnce();
    expect(relaunch).toHaveBeenCalledOnce();
  });

  it("reports up-to-date and actionable errors on manual checks", async () => {
    vi.mocked(check).mockResolvedValueOnce(null).mockRejectedValueOnce(new Error("HTTP 500"));
    const service = TestBed.inject(UpdaterService);

    await service.checkForUpdates();
    expect(service.status()).toBe("up-to-date");
    expect(service.error()).toBeNull();

    await service.checkForUpdates();
    expect(service.status()).toBe("error");
    expect(service.error()).toContain("Impossibile verificare");
    expect(log.record).toHaveBeenCalledWith("error", "updater", expect.any(Error));
  });
});
