import { Injectable, inject } from "@angular/core";
import { TauriService } from "./tauri.service";
import type {
  SyncConfig,
  SyncStatusDto,
  SyncPushResult,
  SyncPullResult,
} from "../models/sync.model";

const SYNC_CONFIG_STORAGE_KEY = "eattura_e2ee_sync_config";

@Injectable({ providedIn: "root" })
export class SyncService {
  private tauri = inject(TauriService);

  /**
   * Load saved sync configuration from localStorage or default values.
   */
  getStoredConfig(): SyncConfig {
    try {
      const saved = localStorage.getItem(SYNC_CONFIG_STORAGE_KEY);
      if (saved) {
        return JSON.parse(saved);
      }
    } catch {
      // Fallback
    }

    return {
      relayUrl: "http://localhost:8080",
      passphrase: "",
      deviceId: "Desktop " + Math.floor(1000 + Math.random() * 9000),
      apiKey: "",
    };
  }

  /**
   * Save sync configuration to localStorage.
   */
  saveStoredConfig(config: SyncConfig): void {
    try {
      localStorage.setItem(SYNC_CONFIG_STORAGE_KEY, JSON.stringify(config));
    } catch {
      // Ignore
    }
  }

  /**
   * Generate an Italian friendly 4-word pairing code / passphrase.
   */
  async generatePairingCode(): Promise<string> {
    return this.tauri.invoke<string>("generate_sync_pairing_code");
  }

  /**
   * Push encrypted local vault changes to the stateless server relay.
   */
  async pushChanges(config: SyncConfig): Promise<SyncPushResult> {
    this.saveStoredConfig(config);
    return this.tauri.invoke<SyncPushResult>("sync_push_changes", { config });
  }

  /**
   * Pull and apply encrypted changes from the stateless server relay.
   */
  async pullChanges(config: SyncConfig): Promise<SyncPullResult> {
    this.saveStoredConfig(config);
    return this.tauri.invoke<SyncPullResult>("sync_pull_changes", { config });
  }

  /**
   * Check relay server connectivity and pending message count.
   */
  async checkRelay(config: SyncConfig): Promise<SyncStatusDto> {
    return this.tauri.invoke<SyncStatusDto>("sync_check_relay", { config });
  }

  /**
   * Full bidirectional synchronization: pushes outgoing changes then pulls incoming.
   */
  async syncAll(config: SyncConfig): Promise<{
    push: SyncPushResult;
    pull: SyncPullResult;
  }> {
    const push = await this.pushChanges(config);
    const pull = await this.pullChanges(config);
    return { push, pull };
  }
}
