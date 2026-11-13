import { TestBed } from "@angular/core/testing";
import { readFileSync } from "node:fs";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { McpService, type McpClient, type McpStatus } from "../../core/services/mcp.service";
import { McpSettingsComponent } from "./mcp-settings.component";

describe("McpSettingsComponent", () => {
  const status: McpStatus = {
    running: true,
    transport: "stdio",
    configPath: "/local/mcp.json",
    description: "Open Eattura before connecting",
    error: null,
  };
  const service = { status: vi.fn(), clientConfig: vi.fn(), scanClients: vi.fn(), installClient: vi.fn() };
  const client: McpClient = {
    id: "codex", name: "Codex", installed: true, configured: false, canInstall: true,
    configPath: "/test/.codex/config.toml", detail: "Ready",
  };
  let component: McpSettingsComponent;

  beforeEach(async () => {
    vi.resetAllMocks();
    service.status.mockResolvedValue(status);
    service.clientConfig.mockResolvedValue('{"mcpServers":{"eattura-open-app-required":{"command":"/test-only/eattura","args":["--mcp-stdio"]}}}');
    service.scanClients.mockResolvedValue([client]);
    service.installClient.mockResolvedValue({ ...client, configured: true, canInstall: false });
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ providers: [
      { provide: McpService, useValue: service },
    ] });
    TestBed.overrideComponent(McpSettingsComponent, { set: {
      templateUrl: undefined,
      styleUrl: undefined,
      template: readFileSync("src/app/features/settings/mcp-settings.component.html", "utf8"),
      styles: [],
    } });
    await TestBed.compileComponents();
    component = TestBed.runInInjectionContext(() => new McpSettingsComponent());
  });

  it("loads status without fetching client configuration", async () => {
    await component.refresh();
    expect(component.status()).toEqual(status);
    expect(component.config()).toBeNull();
    expect(service.clientConfig).not.toHaveBeenCalled();
    expect(service.scanClients).not.toHaveBeenCalled();
  });

  it("reveals configuration only on request and clears it when hidden", async () => {
    await component.toggleConfig();
    expect(component.config()).toContain("test-only");
    await component.toggleConfig();
    expect(component.config()).toBeNull();
    expect(service.clientConfig).toHaveBeenCalledTimes(1);
  });

  it("clears displayed configuration on status refresh", async () => {
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
    expect(element.textContent).toContain("creare e aggiornare bozze");
    expect(element.textContent).toContain("L’invio si esegue manualmente in Eattura");
    expect(element.querySelector("textarea")).toBeNull();
    expect(service.clientConfig).not.toHaveBeenCalled();
  });

  it("renders stdio configuration without an HTTP endpoint or bearer credentials", async () => {
    const fixture = TestBed.createComponent(McpSettingsComponent);
    fixture.detectChanges();
    await fixture.whenStable();
    await fixture.componentInstance.toggleConfig();
    fixture.detectChanges();
    const element: HTMLElement = fixture.nativeElement;
    expect(element.textContent).toContain("stdio");
    expect(element.textContent).toContain("nessuna porta HTTP aperta");
    expect(element.textContent).toContain("stdio non aggira questo limite");
    const config = element.querySelector("textarea")?.value;
    expect(config).toContain("--mcp-stdio");
    expect(config).not.toContain("Authorization");
    expect(config).not.toContain("Bearer");
    expect(config).not.toContain("http://");
  });

  it("scans only on request without revealing credentials or installing anything", async () => {
    await component.scanClients();
    expect(component.clients()).toEqual([client]);
    expect(service.scanClients).toHaveBeenCalledTimes(1);
    expect(service.clientConfig).not.toHaveBeenCalled();
    expect(service.installClient).not.toHaveBeenCalled();
  });

  it("installs directly with one button after displaying the data access warning", async () => {
    const fixture = TestBed.createComponent(McpSettingsComponent);
    fixture.detectChanges();
    await fixture.whenStable();
    await fixture.componentInstance.scanClients();
    fixture.detectChanges();
    const element: HTMLElement = fixture.nativeElement;
    const connect = Array.from(element.querySelectorAll("button")).find((button) => button.textContent?.includes("Installa MCP in Codex"));
    expect(connect).toBeDefined();
    expect(element.textContent).toContain("Il token resta locale");
    expect(element.textContent).toContain("elaborati dal suo fornitore AI");
    expect(service.installClient).not.toHaveBeenCalled();
    connect?.click();
    await fixture.whenStable();
    fixture.detectChanges();
    expect(service.installClient).toHaveBeenCalledWith("codex");
    expect(element.textContent).toContain("MCP installato in Codex");
    expect(element.querySelectorAll("article button")).toHaveLength(0);
    expect(element.querySelector("a")).toBeNull();
    expect(service.clientConfig).not.toHaveBeenCalled();
  });

  it("hides missing clients and never offers guides or installation for unsupported detected apps", async () => {
    const fixture = TestBed.createComponent(McpSettingsComponent);
    service.scanClients.mockResolvedValue([
      { ...client, id: "chatgpt", name: "ChatGPT", configPath: null, canInstall: false, detail: "Local-only server" },
      { ...client, id: "gemini-cli", name: "Gemini CLI", installed: false, canInstall: false },
    ]);
    fixture.detectChanges();
    await fixture.whenStable();
    await fixture.componentInstance.scanClients();
    fixture.detectChanges();
    const element: HTMLElement = fixture.nativeElement;
    expect(element.querySelectorAll("article")).toHaveLength(1);
    expect(element.querySelectorAll("article button")).toHaveLength(0);
    expect(element.textContent).toContain("ChatGPT");
    expect(element.textContent).not.toContain("Gemini CLI");
    expect(element.textContent).not.toContain("Gemini Web");
    expect(element.querySelector("a")).toBeNull();
  });

  it("clears stale results and shows safe messages after failures", async () => {
    await component.scanClients();
    service.scanClients.mockRejectedValueOnce(new Error("secret data"));
    await component.scanClients();
    expect(component.clients()).toBeNull();
    expect(component.error()).not.toContain("secret data");
    service.installClient.mockRejectedValueOnce(new Error("private config"));
    await component.installClient(client);
    expect(component.error()).toContain("non vengono sovrascritte");
    expect(component.error()).not.toContain("private config");
    expect(component.loading()).toBe(false);
  });

  it("prevents duplicate installation while a request is pending", async () => {
    let resolveInstall: ((client: McpClient) => void) | undefined;
    service.installClient.mockReturnValueOnce(new Promise<McpClient>((resolve) => { resolveInstall = resolve; }));
    const pending = component.installClient(client);
    await component.installClient(client);
    expect(service.installClient).toHaveBeenCalledTimes(1);
    resolveInstall?.({ ...client, configured: true });
    await pending;
  });

  it("shows an empty state rather than listing undetected applications", async () => {
    service.scanClients.mockResolvedValue([{ ...client, installed: false }]);
    const fixture = TestBed.createComponent(McpSettingsComponent);
    fixture.detectChanges();
    await fixture.whenStable();
    await fixture.componentInstance.scanClients();
    fixture.detectChanges();
    const element: HTMLElement = fixture.nativeElement;
    expect(element.querySelector("article")).toBeNull();
    expect(element.textContent).toContain("Nessun assistente locale rilevato");
    expect(element.textContent).not.toContain("Codex");
  });

  it("never installs unsupported or undetected clients", async () => {
    await component.installClient({ ...client, installed: false });
    await component.installClient({ ...client, canInstall: false });
    expect(service.installClient).not.toHaveBeenCalled();
  });

  it("removes a client if it is no longer detected after installation", async () => {
    await component.scanClients();
    service.installClient.mockResolvedValueOnce({ ...client, installed: false, configured: false, canInstall: false });
    await component.installClient(client);
    expect(component.clients()).toEqual([]);
    expect(component.message()).toBeNull();
    expect(component.error()).toContain("Configurazione non confermata");
  });
});
