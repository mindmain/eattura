import { ErrorHandler, Injectable, inject } from "@angular/core";
import { invoke, isTauri } from "@tauri-apps/api/core";

export type LogLevel = "info" | "warn" | "error";

export interface LogEntry {
  timestamp: number;
  level: LogLevel;
  source: string;
  message: string;
}

export interface LogSnapshot {
  entries: LogEntry[];
  directory: string | null;
  backendError?: string | null;
}

/** Do not serialize error objects, IPC arguments, invoice payloads, or stacks. */
export function diagnosticMessage(error: unknown): string {
  const message = error instanceof Error ? error.message
    : typeof error === "string" ? error : "Unexpected application error";
  if (/<\?xml|<(?:[\w.-]+:)?FatturaElettronica|-----BEGIN .*PRIVATE KEY/i.test(message)) {
    return "Operation failed; document content omitted from diagnostics";
  }
  return message
    .replace(/(https?:\/\/|imaps?:\/\/|smtps?:\/\/)[^\s/@]+:[^\s/@]+@/gi, "$1[redacted]@")
    .replace(/(["']?(?:password|passwd|token|secret|authorization|api[_-]?key)["']?\s*[=:]\s*)(?:"[^"]*"|'[^']*'|(?:Bearer|Basic)\s+[^\s,;]+|[^\s,;]+)/gi, "$1[redacted]")
    .slice(0, 2048);
}

@Injectable({ providedIn: "root" })
export class AppLogService {
  private localEntries: LogEntry[] = [];
  private persistenceError: string | null = null;

  private async nativeRequest<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    let timeout: ReturnType<typeof setTimeout> | undefined;
    try {
      return await Promise.race([
        invoke<T>(command, args),
        new Promise<never>((_, reject) => {
          timeout = setTimeout(() => reject(new Error("Desktop diagnostic request timed out after 5 seconds")), 5000);
        }),
      ]);
    } finally {
      if (timeout !== undefined) clearTimeout(timeout);
    }
  }

  /** Use invoke directly so a logging failure cannot recursively log itself. */
  async record(level: LogLevel, source: string, message: unknown): Promise<void> {
    const entry: LogEntry = { timestamp: Date.now(), level, source, message: diagnosticMessage(message) };
    this.localEntries = [entry, ...this.localEntries].slice(0, 500);
    if (!isTauri()) return;
    try {
      await this.nativeRequest<void>("write_app_log", { level, source, message: entry.message });
      this.localEntries = this.localEntries.filter((pending) => pending !== entry);
    } catch (error) {
      this.persistenceError = diagnosticMessage(error);
      console.warn("Eattura: unable to persist diagnostic event");
    }
  }

  async snapshot(): Promise<LogSnapshot> {
    if (!isTauri()) {
      return this.localSnapshot("Collegamento all’app desktop non disponibile: il registro è temporaneo e la sincronizzazione PEC richiede il backend desktop.");
    }
    try {
      const snapshot = await this.nativeRequest<LogSnapshot>("get_app_logs");
      if (!snapshot || !Array.isArray(snapshot.entries)) throw new Error("Invalid diagnostic response from desktop backend");
      return {
        ...snapshot,
        entries: [...this.localEntries, ...snapshot.entries].sort((a, b) => b.timestamp - a.timestamp).slice(0, 500),
        backendError: this.persistenceError,
      };
    } catch (error) {
      return this.localSnapshot(diagnosticMessage(error));
    }
  }

  private localSnapshot(backendError: string): LogSnapshot {
    return { entries: [...this.localEntries], directory: null, backendError };
  }

  async openDirectory(): Promise<void> {
    return invoke<void>("open_app_log_directory");
  }
}

@Injectable()
export class AppErrorHandler implements ErrorHandler {
  private readonly log = inject(AppLogService);

  handleError(error: unknown): void {
    void this.log.record("error", "frontend", error);
    console.error(error);
  }
}
