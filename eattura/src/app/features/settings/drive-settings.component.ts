import { DatePipe } from "@angular/common";
import { ChangeDetectionStrategy, Component, inject, signal } from "@angular/core";
import { DriveService } from "../../core/services/drive.service";

@Component({
  selector: "app-drive-settings",
  standalone: true,
  imports: [DatePipe],
  templateUrl: "./drive-settings.component.html",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DriveSettingsComponent {
  readonly drive = inject(DriveService);
  readonly credentials = signal<File | null>(null);

  chooseCredentials(event: Event): void {
    if (event.target instanceof HTMLInputElement) {
      this.credentials.set(event.target.files?.item(0) ?? null);
    }
  }

  async connect(input: HTMLInputElement): Promise<void> {
    const file = this.credentials();
    if (!file) return;
    this.credentials.set(null);
    input.value = "";
    await this.drive.connect(file);
  }

  actionLabel(action: string | null): string {
    switch (action) {
      case "uploaded":
        return "Backup aggiornato su Google Drive";
      case "downloaded":
        return "Backup scaricato da Google Drive";
      case "merged":
        return "Archivi uniti e backup aggiornato su Google Drive";
      case "up_to_date":
        return "Archivio già allineato con Google Drive";
      default:
        return "In attesa della prima sincronizzazione";
    }
  }
}
