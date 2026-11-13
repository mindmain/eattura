import { ChangeDetectionStrategy, Component, inject, signal } from "@angular/core";
import { DriveService } from "../../core/services/drive.service";

@Component({
  selector: "app-drive-settings",
  standalone: true,
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
}
