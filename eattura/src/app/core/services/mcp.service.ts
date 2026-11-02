import { Injectable, inject } from "@angular/core";
import { isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { TauriService } from "./tauri.service";
import { InvoiceStore } from "../stores/invoice.store";
import { NotificationService } from "./notification.service";
import { AppLogService } from "./app-log.service";

export interface McpStatus {
  running: boolean;
  endpoint: string;
  configPath: string;
  description: string;
  error: string | null;
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

  async start(): Promise<void> {
    if (!isTauri() || this.listening) return;
    this.listening = true;
    const generation = ++this.generation;
    try {
      const unlisten = await listen("mcp-draft-created", () => void this.draftCreated());
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

  private async draftCreated(): Promise<void> {
    try {
      await this.invoices.refresh();
      this.notifications.success("Nuova bozza creata tramite MCP: controllala prima di inviarla.");
    } catch (error) {
      void this.log.record("error", "mcp", error);
      this.notifications.error("Bozza MCP salvata, ma elenco non aggiornato. Riapri Eattura per visualizzarla.");
    }
  }
}
