import { DatePipe } from "@angular/common";
import { ChangeDetectionStrategy, Component, OnInit, computed, inject, signal } from "@angular/core";
import { AppLogService, LogLevel } from "../../core/services/app-log.service";

@Component({
  selector: "app-diagnostics",
  standalone: true,
  imports: [DatePipe],
  templateUrl: "./diagnostics.component.html",
  styleUrl: "./diagnostics.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DiagnosticsComponent implements OnInit {
  private readonly log = inject(AppLogService);
  protected readonly snapshot = signal<Awaited<ReturnType<AppLogService["snapshot"]>>>({ entries: [], directory: null });
  protected readonly level = signal<LogLevel | "all">("all");
  protected readonly loading = signal(false);
  protected readonly error = signal("");
  protected readonly labels = { info: "Informazione", warn: "Avviso", error: "Errore" };
  protected readonly entries = computed(() => this.snapshot().entries.filter(
    (entry) => this.level() === "all" || entry.level === this.level()
  ));

  ngOnInit(): void {
    void this.refresh();
  }

  protected setLevel(value: string): void {
    if (value === "all" || value === "info" || value === "warn" || value === "error") {
      this.level.set(value);
    }
  }

  protected async refresh(): Promise<void> {
    if (this.loading()) return;
    this.loading.set(true);
    this.error.set("");
    try {
      this.snapshot.set(await this.log.snapshot());
    } catch {
      this.error.set("Impossibile caricare il registro. Riprova tra poco.");
    } finally {
      this.loading.set(false);
    }
  }

  protected async openDirectory(): Promise<void> {
    this.error.set("");
    try {
      await this.log.openDirectory();
    } catch (error) {
      this.error.set("Impossibile aprire la cartella dei log.");
      void this.log.record("error", "diagnostics", error);
    }
  }
}
