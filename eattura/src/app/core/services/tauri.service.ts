import { Injectable } from "@angular/core";
import { invoke } from "@tauri-apps/api/core";

/**
 * Generic wrapper around Tauri's invoke API.
 * Provides type-safe IPC calls to Rust backend commands.
 */
@Injectable({ providedIn: "root" })
export class TauriService {
  /**
   * Invoke a Tauri command with optional arguments.
   * Automatically handles serialization/deserialization and error wrapping.
   */
  async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    try {
      return await invoke<T>(command, args);
    } catch (error) {
      throw typeof error === "string" ? new Error(error) : error;
    }
  }
}
