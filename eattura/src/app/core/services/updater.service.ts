import { Injectable, computed, inject, signal } from "@angular/core";
import { isTauri } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { AppLogService } from "./app-log.service";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "downloading"
  | "ready"
  | "installing"
  | "up-to-date"
  | "error";

@Injectable({ providedIn: "root" })
export class UpdaterService {
  private readonly log = inject(AppLogService);

  private readonly state = signal<UpdateStatus>("idle");
  private readonly version = signal<string | null>(null);
  private readonly notes = signal<string | null>(null);
  private readonly percent = signal<number | null>(null);
  private readonly problem = signal<string | null>(null);
  private readonly bannerDismissed = signal(false);

  private pendingUpdate: Update | null = null;
  private started = false;

  readonly status = this.state.asReadonly();
  readonly availableVersion = this.version.asReadonly();
  readonly releaseNotes = this.notes.asReadonly();
  readonly progress = this.percent.asReadonly();
  readonly error = this.problem.asReadonly();
  readonly busy = computed(() => {
    const current = this.state();
    return current === "checking" || current === "downloading" || current === "installing";
  });
  readonly bannerVisible = computed(() => {
    const current = this.state();
    return !this.bannerDismissed() && (current === "downloading" || current === "ready" || current === "installing");
  });

  get desktop(): boolean {
    return isTauri();
  }

  /** Automatically checks GitHub Releases once on startup and pre-downloads any update. */
  async start(): Promise<void> {
    if (!isTauri() || this.started) return;
    this.started = true;
    await this.runCheck(true);
  }

  /** Manually checks for updates from the About screen. */
  async checkForUpdates(): Promise<void> {
    if (!isTauri() || this.busy()) return;
    this.bannerDismissed.set(false);
    await this.runCheck(false);
  }

  /** Installs the verified update package and restarts the desktop application. */
  async installAndRestart(): Promise<void> {
    if (!isTauri() || !this.pendingUpdate || this.state() !== "ready") return;
    const update = this.pendingUpdate;
    this.problem.set(null);
    this.state.set("installing");
    try {
      await update.install();
      this.pendingUpdate = null;
      await relaunch();
    } catch (error) {
      this.state.set("ready");
      this.problem.set("Installazione dell’aggiornamento non riuscita. Riprova tra poco o scarica la nuova versione da GitHub.");
      void this.log.record("error", "updater", error);
    }
  }

  dismissBanner(): void {
    this.bannerDismissed.set(true);
  }

  private async runCheck(silent: boolean): Promise<void> {
    this.problem.set(null);
    this.state.set("checking");
    try {
      const update = await check();
      if (!update) {
        await this.clearPending();
        this.version.set(null);
        this.notes.set(null);
        this.percent.set(null);
        this.state.set(silent ? "idle" : "up-to-date");
        return;
      }

      if (this.pendingUpdate && this.pendingUpdate.version === update.version) {
        await update.close().catch(() => undefined);
        this.state.set("ready");
        return;
      }

      await this.clearPending();
      this.pendingUpdate = update;
      this.version.set(update.version);
      this.notes.set(update.body?.trim() || null);
      this.percent.set(null);
      this.state.set("downloading");
      void this.log.record("info", "updater", `Update ${update.version} available; downloading package`);

      let totalBytes = 0;
      let downloadedBytes = 0;
      await update.download((event: DownloadEvent) => {
        if (event.event === "Started") {
          totalBytes = event.data.contentLength ?? 0;
          downloadedBytes = 0;
          this.percent.set(totalBytes > 0 ? 0 : null);
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          if (totalBytes > 0) {
            this.percent.set(Math.min(100, Math.round((downloadedBytes / totalBytes) * 100)));
          }
        } else if (event.event === "Finished") {
          this.percent.set(100);
        }
      });

      this.state.set("ready");
      void this.log.record("info", "updater", `Update ${update.version} downloaded and verified`);
    } catch (error) {
      await this.clearPending();
      this.percent.set(null);
      if (silent) {
        this.state.set("idle");
        void this.log.record("warn", "updater", error);
      } else {
        this.state.set("error");
        this.problem.set("Impossibile verificare o scaricare gli aggiornamenti. Controlla la connessione e riprova.");
        void this.log.record("error", "updater", error);
      }
    }
  }

  private async clearPending(): Promise<void> {
    const previous = this.pendingUpdate;
    this.pendingUpdate = null;
    if (previous) {
      await previous.close().catch(() => undefined);
    }
  }
}
