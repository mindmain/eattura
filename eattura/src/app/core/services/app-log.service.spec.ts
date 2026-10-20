import { Injector } from "@angular/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { AppErrorHandler, AppLogService, diagnosticMessage } from "./app-log.service";
import { TauriService } from "./tauri.service";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), isTauri: vi.fn() }));

describe("Application diagnostics", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(isTauri).mockReturnValue(true);
  });

  afterEach(() => { vi.restoreAllMocks(); vi.useRealTimers(); });

  it("redacts credentials and omits document payloads and unknown objects", () => {
    expect(diagnosticMessage(new Error("password='secret value'; token=abc")))
      .toBe("password=[redacted]; token=[redacted]");
    expect(diagnosticMessage("imap://alice:secret@mail.test failed"))
      .toBe("imap://[redacted]@mail.test failed");
    expect(diagnosticMessage('"password": "secret", Authorization: Bearer abc123'))
      .toBe('"password": [redacted], Authorization: [redacted]');
    expect(diagnosticMessage("Invalid <?xml version='1.0'?><FatturaElettronica>private"))
      .not.toContain("private");
    expect(diagnosticMessage({ password: "secret" })).not.toContain("secret");
  });

  it("does not retry or throw when logging itself fails", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    vi.mocked(invoke).mockRejectedValue("disk unavailable");
    await expect(new AppLogService().record("error", "test", "Failure")).resolves.toBeUndefined();
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(warn).toHaveBeenCalledOnce();
  });

  it("logs failed commands without their arguments and preserves the original rejection", async () => {
    const record = vi.fn().mockResolvedValue(undefined);
    const injector = Injector.create({ providers: [TauriService, { provide: AppLogService, useValue: { record } }] });
    vi.mocked(invoke).mockRejectedValue("Keyring unavailable");
    await expect(injector.get(TauriService).invoke("set_pec_password", { password: "private" }))
      .rejects.toThrow("Keyring unavailable");
    expect(record).toHaveBeenCalledWith("error", "set_pec_password", "Unable to save PEC credentials");
    expect(JSON.stringify(record.mock.calls)).not.toContain("private");
  });

  it("records unhandled frontend errors", () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    const record = vi.fn().mockResolvedValue(undefined);
    const injector = Injector.create({ providers: [AppErrorHandler, { provide: AppLogService, useValue: { record } }] });
    const error = new Error("Unexpected failure");
    injector.get(AppErrorHandler).handleError(error);
    expect(record).toHaveBeenCalledWith("error", "frontend", error);
  });

  it("keeps browser errors visible instead of returning an empty diagnostic history", async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    const log = new AppLogService();
    await log.record("info", "app", "Started");
    expect(await log.snapshot()).toEqual({
      entries: [expect.objectContaining({ source: "app", message: "Started" })],
      directory: null,
      backendError: expect.stringContaining("desktop"),
    });
    expect(invoke).not.toHaveBeenCalled();
  });

  it("retains errors when both persistence and the native log reader fail", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    vi.mocked(invoke).mockRejectedValue("Native backend unavailable");
    const log = new AppLogService();
    await log.record("error", "pec-monitor", "PEC status unavailable");
    const snapshot = await log.snapshot();
    expect(snapshot.entries[0].message).toBe("PEC status unavailable");
    expect(snapshot.backendError).toBe("Native backend unavailable");
  });

  it("shows local diagnostics when the native log command never settles", async () => {
    vi.useFakeTimers();
    vi.mocked(isTauri).mockReturnValue(false);
    const log = new AppLogService();
    await log.record("error", "pec-monitor", "Pending request");
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}));
    const pending = log.snapshot();
    await vi.advanceTimersByTimeAsync(5000);
    const snapshot = await pending;
    expect(snapshot.entries[0].message).toBe("Pending request");
    expect(snapshot.backendError).toContain("timed out");
  });
});
