export interface SyncConfig {
  relayUrl: string;
  passphrase: string;
  deviceId: string;
  apiKey?: string;
}

export interface SyncStatusDto {
  topic: string;
  pendingMessages: number;
  serverHealthy: boolean;
  message: string;
}

export interface SyncPushResult {
  success: boolean;
  invoicesPushed: number;
  clientsPushed: number;
  topic: string;
  messageId: string;
}

export interface SyncPullResult {
  success: boolean;
  envelopesReceived: number;
  invoicesProcessed: number;
  clientsProcessed: number;
  errors: string[];
}
