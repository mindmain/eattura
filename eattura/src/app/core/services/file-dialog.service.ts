import { Injectable } from "@angular/core";
import { open } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";

export interface OpenDirectoryResult {
  path: string;
}

@Injectable({ providedIn: "root" })
export class FileDialogService {
  async pickAndReadXmlFile(): Promise<{ filename: string; content: string } | null> {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Fattura XML", extensions: ["xml"] }],
    });
    if (!selected || Array.isArray(selected)) {
      return null;
    }
    const content = await readTextFile(selected);
    return { filename: selected, content };
  }

  async pickDirectory(): Promise<string | null> {
    const selected = await open({ directory: true, multiple: false });
    if (!selected || Array.isArray(selected)) {
      return null;
    }
    return selected;
  }
}
