import { Injector } from "@angular/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { McpService } from "./mcp.service";
import { TauriService } from "./tauri.service";
import { InvoiceStore } from "../stores/invoice.store";
import { NotificationService } from "./notification.service";
import { AppLogService } from "./app-log.service";

const runtime = vi.hoisted(() => ({ desktop: true, listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => runtime.desktop }));
vi.mock("@tauri-apps/api/event", () => ({ listen: runtime.listen }));

describe("McpService", () => {
  let service: McpService;
  const invoke = vi.fn();
  const refresh = vi.fn();
  const notify = { success: vi.fn(), error: vi.fn() };
  const record = vi.fn();

  beforeEach(() => {
    vi.resetAllMocks();
    runtime.desktop = true;
    refresh.mockResolvedValue([]);
    service = Injector.create({ providers: [
      McpService,
      { provide: TauriService, useValue: { invoke } },
      { provide: InvoiceStore, useValue: { refresh } },
      { provide: NotificationService, useValue: notify },
      { provide: AppLogService, useValue: { record } },
    ] }).get(McpService);
  });

  it("does not attempt a desktop connection in a browser", async () => {
    runtime.desktop = false;
    expect(await service.status()).toBeNull();
    await service.start();
    expect(invoke).not.toHaveBeenCalled();
    expect(runtime.listen).not.toHaveBeenCalled();
  });

  it("refreshes local invoice state after draft creation and releases its listener", async () => {
    const unlisten = vi.fn();
    let onDraft: (() => void) | undefined;
    runtime.listen.mockImplementation(async (_event: string, callback: () => void) => {
      onDraft = callback;
      return unlisten;
    });
    await service.start();
    await service.start();
    expect(runtime.listen).toHaveBeenCalledTimes(1);
    onDraft?.();
    await vi.waitFor(() => expect(notify.success).toHaveBeenCalled());
    expect(refresh).toHaveBeenCalledTimes(1);
    expect(notify.success.mock.calls[0][0]).toContain("controllala prima di inviarla");
    service.stop();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("releases listeners that finish registering after shutdown", async () => {
    const unlisten = vi.fn();
    let resolve: ((callback: () => void) => void) | undefined;
    runtime.listen.mockReturnValue(new Promise<() => void>((done) => { resolve = done; }));
    const pending = service.start();
    service.stop();
    resolve?.(unlisten);
    await pending;
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it("only requests client credentials through the explicit configuration method", async () => {
    invoke.mockResolvedValue({ running: true });
    await service.status();
    expect(invoke).toHaveBeenCalledWith("get_mcp_status");
    expect(invoke).not.toHaveBeenCalledWith("get_mcp_client_config");
    invoke.mockResolvedValue({ mcpServers: {} });
    expect(await service.clientConfig()).toBe(JSON.stringify({ mcpServers: {} }, null, 2));
    expect(invoke).toHaveBeenLastCalledWith("get_mcp_client_config");
  });

  it("does not retain an old registration when stopped and restarted before it resolves", async () => {
    const staleUnlisten = vi.fn();
    const currentUnlisten = vi.fn();
    let resolveOld: ((callback: () => void) => void) | undefined;
    runtime.listen.mockReturnValueOnce(new Promise<() => void>((done) => { resolveOld = done; }));
    runtime.listen.mockResolvedValueOnce(currentUnlisten);
    const oldStart = service.start();
    service.stop();
    await service.start();
    resolveOld?.(staleUnlisten);
    await oldStart;
    expect(staleUnlisten).toHaveBeenCalledTimes(1);
    expect(currentUnlisten).not.toHaveBeenCalled();
    service.stop();
    expect(currentUnlisten).toHaveBeenCalledTimes(1);
  });
});
