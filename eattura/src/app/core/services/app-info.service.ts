import { Injectable } from "@angular/core";
import { getVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

@Injectable({ providedIn: "root" })
export class AppInfoService {
  readonly repositoryUrl = "https://github.com/mindmain/eattura";
  readonly websiteUrl = "https://ocin.sh";

  async version(): Promise<string | null> {
    return isTauri() ? getVersion() : null;
  }

  /** Browser anchors keep their native behavior; desktop links use the system browser. */
  async openLink(event: MouseEvent, url: string): Promise<void> {
    if (!isTauri()) return;
    event.preventDefault();
    await openUrl(url);
  }
}
