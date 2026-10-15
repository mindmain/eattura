import { describe, it, expect, beforeEach, vi } from "vitest";
import { TestBed } from "@angular/core/testing";
import { SyncService } from "./sync.service";
import { TauriService } from "./tauri.service";

describe("SyncService", () => {
  let service: SyncService;
  let tauriMock: { invoke: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    localStorage.clear();
    tauriMock = {
      invoke: vi.fn(),
    };

    TestBed.configureTestingModule({
      providers: [
        SyncService,
        { provide: TauriService, useValue: tauriMock },
      ],
    });
    service = TestBed.inject(SyncService);
  });

  it("should get default config when localStorage is empty", () => {
    const cfg = service.getStoredConfig();
    expect(cfg.relayUrl).toBe("http://localhost:8080");
    expect(cfg.passphrase).toBe("");
    expect(cfg.deviceId).toContain("Desktop");
  });

  it("should persist and retrieve custom config in localStorage", () => {
    const custom = {
      relayUrl: "https://relay.eattura.io",
      passphrase: "sole-luna-stella-1234",
      deviceId: "MacBook-Air",
      apiKey: "secret-key",
    };

    service.saveStoredConfig(custom);
    const recovered = service.getStoredConfig();
    expect(recovered).toEqual(custom);
  });

  it("generatePairingCode invoca generate_sync_pairing_code via Tauri", async () => {
    tauriMock.invoke.mockResolvedValue("parola-casa-sole-mare");
    const code = await service.generatePairingCode();
    expect(tauriMock.invoke).toHaveBeenCalledWith("generate_sync_pairing_code");
    expect(code).toBe("parola-casa-sole-mare");
  });

  it("pushChanges e pullChanges invocano i rispettivi comandi Tauri e salvano la config", async () => {
    const config = {
      relayUrl: "https://relay.eattura.io",
      passphrase: "test",
      deviceId: "device-1",
      apiKey: "key",
    };
    tauriMock.invoke.mockResolvedValue({ success: true, count: 1 });

    await service.pushChanges(config);
    expect(tauriMock.invoke).toHaveBeenCalledWith("sync_push_changes", { config });
    expect(service.getStoredConfig().deviceId).toBe("device-1");

    await service.pullChanges(config);
    expect(tauriMock.invoke).toHaveBeenCalledWith("sync_pull_changes", { config });
  });
});
