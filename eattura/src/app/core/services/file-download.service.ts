import { Injectable, inject, PLATFORM_ID } from "@angular/core";
import { isPlatformBrowser } from "@angular/common";

@Injectable({ providedIn: "root" })
export class FileDownloadService {
  private platformId = inject(PLATFORM_ID);

  downloadTextFile(content: string, filename: string, mimeType = "application/xml"): void {
    if (!isPlatformBrowser(this.platformId)) {
      return;
    }

    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = filename;
    anchor.click();
    URL.revokeObjectURL(url);
  }

  downloadBinaryFile(
    data: Uint8Array | ArrayBuffer | Blob,
    filename: string,
    mimeType = "application/pdf"
  ): void {
    if (!isPlatformBrowser(this.platformId)) {
      return;
    }

    const blob = data instanceof Blob ? data : new Blob([data as any], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = filename;
    anchor.click();
    URL.revokeObjectURL(url);
  }
}
