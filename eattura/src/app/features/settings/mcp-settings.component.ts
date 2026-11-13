import { ChangeDetectionStrategy, Component, OnInit, inject, signal } from "@angular/core";
import { McpService, type McpClient, type McpStatus } from "../../core/services/mcp.service";

@Component({
  selector: "app-mcp-settings",
  standalone: true,
  templateUrl: "./mcp-settings.component.html",
  styleUrl: "./mcp-settings.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class McpSettingsComponent implements OnInit {
  private readonly mcp = inject(McpService);
  readonly status = signal<McpStatus | null>(null);
  readonly loading = signal(false);
  readonly error = signal<string | null>(null);
  readonly config = signal<string | null>(null);
  readonly clients = signal<McpClient[] | null>(null);
  readonly message = signal<string | null>(null);

  ngOnInit(): void {
    void this.refresh();
  }

  async refresh(): Promise<void> {
    if (this.loading()) return;
    this.loading.set(true);
    this.error.set(null);
    this.config.set(null);
    this.message.set(null);
    try {
      this.status.set(await this.mcp.status());
    } catch {
      this.status.set(null);
      this.error.set("Impossibile leggere lo stato MCP. Controlla il registro attività.");
    } finally {
      this.loading.set(false);
    }
  }

  async scanClients(): Promise<void> {
    if (this.loading()) return;
    this.loading.set(true);
    this.error.set(null);
    this.config.set(null);
    this.message.set(null);
    try {
      this.clients.set((await this.mcp.scanClients()).filter((client) => client.installed));
    } catch {
      this.clients.set(null);
      this.error.set("Scansione non riuscita. Apri Eattura e controlla il registro attività.");
    } finally {
      this.loading.set(false);
    }
  }

  async installClient(client: McpClient): Promise<void> {
    if (!client.installed || !client.canInstall || this.loading()) return;
    this.loading.set(true);
    this.error.set(null);
    this.message.set(null);
    try {
      const updated = await this.mcp.installClient(client.id);
      this.clients.update((clients) => clients?.map((item) => item.id === updated.id ? updated : item).filter((item) => item.installed) ?? null);
      if (updated.configured) {
        this.message.set(`MCP installato in ${client.name}. Riavvia il client, verifica MCP e lascia Eattura aperta.`);
      } else {
        this.error.set("Configurazione non confermata. Ripeti la scansione e verifica il file del client.");
      }
    } catch {
      this.error.set("Collegamento non riuscito. Chiudi il client, controlla il file di configurazione e ripeti la scansione. Le configurazioni non valide o in conflitto non vengono sovrascritte.");
    } finally {
      this.loading.set(false);
    }
  }

  async toggleConfig(): Promise<void> {
    if (this.config()) {
      this.config.set(null);
      return;
    }
    if (this.loading()) return;
    this.loading.set(true);
    this.error.set(null);
    try {
      this.config.set(await this.mcp.clientConfig());
    } catch {
      this.error.set("Configurazione MCP non disponibile. Apri Eattura e controlla che il server sia attivo.");
    } finally {
      this.loading.set(false);
    }
  }
}
