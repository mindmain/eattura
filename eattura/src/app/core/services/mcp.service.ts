import { Injectable, inject } from "@angular/core";
import { isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { TauriService } from "./tauri.service";
import { InvoiceStore } from "../stores/invoice.store";
import { NotificationService } from "./notification.service";
import { AppLogService } from "./app-log.service";

export interface McpStatus {
  running: boolean;
  transport: "stdio";
  configPath: string;
  description: string;
  error: string | null;
}

export type McpClientId = "codex" | "claude-code" | "gemini-cli" | "claude-desktop" | "chatgpt";

export interface McpClient {
  id: McpClientId;
  name: string;
  installed: boolean;
  configured: boolean;
  canInstall: boolean;
  configPath: string | null;
  detail: string;
}

@Injectable({ providedIn: "root" })
export class McpService {
  private readonly tauri = inject(TauriService);
  private readonly invoices = inject(InvoiceStore);
  private readonly notifications = inject(NotificationService);
  private readonly log = inject(AppLogService);
  private unlisten?: UnlistenFn;
  private listening = false;
  private generation = 0;

  async status(): Promise<McpStatus | null> {
    return isTauri() ? this.tauri.invoke<McpStatus>("get_mcp_status") : null;
  }

  async clientConfig(): Promise<string> {
    const config = await this.tauri.invoke<unknown>("get_mcp_client_config");
    return JSON.stringify(config, null, 2);
  }

  async scanClients(): Promise<McpClient[]> {
    return isTauri() ? this.tauri.invoke<McpClient[]>("scan_mcp_clients") : [];
  }

  async installClient(clientId: McpClientId): Promise<McpClient> {
    if (!isTauri()) throw new Error("Apri Eattura per collegare un client MCP.");
    return this.tauri.invoke<McpClient>("install_mcp_client", { clientId });
  }

  async start(): Promise<void> {
    if (!isTauri() || this.listening) return;
    this.listening = true;
    const generation = ++this.generation;
    try {
      const unlisten = await listen<{ id: string; previousId: string | null }>(
        "mcp-draft-saved",
        ({ payload }) => void this.draftSaved(payload),
      );
      if (this.listening && this.generation === generation) this.unlisten = unlisten;
      else unlisten();
    } catch (error) {
      if (this.generation === generation) this.listening = false;
      void this.log.record("error", "mcp", error);
    }
  }

  stop(): void {
    this.listening = false;
    this.generation++;
    this.unlisten?.();
    this.unlisten = undefined;
  }

  private async draftSaved(draft: { id: string; previousId: string | null }): Promise<void> {
    this.invoices.announceMcpDraft(draft);
    try {
      await this.invoices.refresh();
      this.notifications.success("Bozza salvata tramite MCP: controllala prima di inviarla.");
    } catch (error) {
      void this.log.record("error", "mcp", error);
      this.notifications.error("Bozza MCP salvata, ma elenco non aggiornato. Riapri Eattura per visualizzarla.");
    }
  }
}
