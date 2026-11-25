import { ChangeDetectionStrategy, Component, OnInit, inject, signal } from "@angular/core";
import { RouterLink } from "@angular/router";
import { LucideAngularModule, Download, ExternalLink, FileCheck2, Github, Globe, HardDrive, Mail, RefreshCw } from "lucide-angular";
import { AppInfoService } from "../../core/services/app-info.service";
import { AppLogService } from "../../core/services/app-log.service";
import { UpdaterService } from "../../core/services/updater.service";

@Component({
  selector: "app-about",
  standalone: true,
  imports: [RouterLink, LucideAngularModule],
  templateUrl: "./about.component.html",
  styleUrl: "./about.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AboutComponent implements OnInit {
  protected readonly app = inject(AppInfoService);
  protected readonly updater = inject(UpdaterService);
  private readonly log = inject(AppLogService);
  protected readonly version = signal<string | null>(null);
  protected readonly versionStatus = signal("Caricamento versione…");
  protected readonly error = signal("");
  protected readonly icons = { Download, ExternalLink, FileCheck2, Github, Globe, HardDrive, Mail, RefreshCw };

  async ngOnInit(): Promise<void> {
    try {
      this.version.set(await this.app.version());
      this.versionStatus.set("Anteprima browser");
    } catch (error) {
      this.versionStatus.set("Versione non disponibile");
      void this.log.record("error", "about", error);
    }
  }

  protected async openLink(event: MouseEvent, url: string): Promise<void> {
    this.error.set("");
    try {
      await this.app.openLink(event, url);
    } catch (error) {
      this.error.set("Impossibile aprire il link nel browser. Riprova tra poco.");
      void this.log.record("error", "about", error);
    }
  }
}
