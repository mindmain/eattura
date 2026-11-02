import { TestBed } from "@angular/core/testing";
import { readFileSync } from "node:fs";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { McpService, type McpStatus } from "../../core/services/mcp.service";
import { McpSettingsComponent } from "./mcp-settings.component";

describe("McpSettingsComponent", () => {
  const status: McpStatus = {
    running: true,
    endpoint: "http://127.0.0.1:18765/mcp",
    configPath: "/local/mcp.json",
    description: "Open Eattura before connecting",
    error: null,
  };
  const service = { status: vi.fn(), clientConfig: vi.fn() };
  let component: McpSettingsComponent;

  beforeEach(async () => {
    vi.resetAllMocks();
    service.status.mockResolvedValue(status);
    service.clientConfig.mockResolvedValue('{"privateToken":"test-only"}');
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ providers: [{ provide: McpService, useValue: service }] });
    TestBed.overrideComponent(McpSettingsComponent, { set: {
      templateUrl: undefined,
      styleUrl: undefined,
      template: readFileSync("src/app/features/settings/mcp-settings.component.html", "utf8"),
      styles: [],
    } });
    await TestBed.compileComponents();
    component = TestBed.runInInjectionContext(() => new McpSettingsComponent());
  });

  it("loads status without revealing or fetching the token", async () => {
    await component.refresh();
    expect(component.status()).toEqual(status);
    expect(component.config()).toBeNull();
    expect(service.clientConfig).not.toHaveBeenCalled();
  });

  it("reveals configuration only on request and clears it when hidden", async () => {
    await component.toggleConfig();
    expect(component.config()).toContain("test-only");
    await component.toggleConfig();
    expect(component.config()).toBeNull();
    expect(service.clientConfig).toHaveBeenCalledTimes(1);
  });

  it("clears displayed credentials on status refresh", async () => {
    await component.toggleConfig();
    await component.refresh();
    expect(component.config()).toBeNull();
  });

  it("shows a safe failure message without displaying backend details", async () => {
    service.clientConfig.mockRejectedValueOnce(new Error("sensitive backend data"));
    await component.toggleConfig();
    expect(component.error()).toContain("Apri Eattura");
    expect(component.error()).not.toContain("sensitive");
    expect(component.config()).toBeNull();
    expect(component.loading()).toBe(false);
  });

  it("renders the desktop requirement and never displays an invoice send action", async () => {
    const fixture = TestBed.createComponent(McpSettingsComponent);
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
    const element: HTMLElement = fixture.nativeElement;
    expect(element.textContent).toContain("Apri Eattura e lasciala aperta");
    expect(element.textContent).toContain("Non possono inviare fatture");
    expect(element.querySelector("textarea")).toBeNull();
    expect(service.clientConfig).not.toHaveBeenCalled();
  });
});
