import { Injectable, inject } from "@angular/core";
import { invoke } from "@tauri-apps/api/core";
import { AppLogService } from "./app-log.service";

/**
 * Generic wrapper around Tauri's invoke API.
 * Provides type-safe IPC calls to Rust backend commands.
 */
@Injectable({ providedIn: "root" })
export class TauriService {
  private readonly log = inject(AppLogService);
  /**
   * Invoke a Tauri command with optional arguments.
   * Automatically handles serialization/deserialization and error wrapping.
   */
  async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    try {
      const result = await invoke<T>(command, args);
      if (/^(create_|update_|delete_|set_|send_|import_|export_|register_|switch_|rebuild_|sync_pec$)/.test(command)) {
        void this.log.record("info", command, "Operation completed");
      }
      return result;
    } catch (error) {
      // Password commands must not include even their error details in diagnostics.
      void this.log.record("error", command, command === "set_pec_password" ? "Unable to save PEC credentials" : error);
      throw typeof error === "string" ? new Error(error) : error;
    }
  }
}
