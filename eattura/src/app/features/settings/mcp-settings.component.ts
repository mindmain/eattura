import { ChangeDetectionStrategy, Component, OnInit, inject, signal } from "@angular/core";
import { McpService, type McpStatus } from "../../core/services/mcp.service";

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

  ngOnInit(): void {
    void this.refresh();
  }

  async refresh(): Promise<void> {
    if (this.loading()) return;
    this.loading.set(true);
    this.error.set(null);
    this.config.set(null);
    try {
      this.status.set(await this.mcp.status());
    } catch {
      this.status.set(null);
      this.error.set("Impossibile leggere lo stato MCP. Controlla il registro attività.");
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
