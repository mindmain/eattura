import { beforeEach, describe, expect, it, vi } from "vitest";
import { getVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AppInfoService } from "./app-info.service";

vi.mock("@tauri-apps/api/app", () => ({ getVersion: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

describe("AppInfoService", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(isTauri).mockReturnValue(true);
  });

  it("reads the installed app version rather than a hardcoded value", async () => {
    vi.mocked(getVersion).mockResolvedValue("2.3.4");
    expect(await new AppInfoService().version()).toBe("2.3.4");
  });

  it.each(["repositoryUrl", "websiteUrl"] as const)("opens %s through the native browser opener", async (key) => {
    const service = new AppInfoService();
    const event = new MouseEvent("click", { cancelable: true });
    await service.openLink(event, service[key]);
    expect(event.defaultPrevented).toBe(true);
    expect(openUrl).toHaveBeenCalledWith(service[key]);
  });

  it("keeps standard anchor navigation in the browser preview", async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    const service = new AppInfoService();
    const event = new MouseEvent("click", { cancelable: true });
    await service.openLink(event, service.websiteUrl);
    expect(event.defaultPrevented).toBe(false);
    expect(openUrl).not.toHaveBeenCalled();
    expect(await service.version()).toBeNull();
    expect(getVersion).not.toHaveBeenCalled();
  });

  it("propagates opener failures so the page can display an actionable error", async () => {
    vi.mocked(openUrl).mockRejectedValue(new Error("Browser unavailable"));
    const service = new AppInfoService();
    await expect(service.openLink(new MouseEvent("click", { cancelable: true }), service.repositoryUrl))
      .rejects.toThrow("Browser unavailable");
  });
});
