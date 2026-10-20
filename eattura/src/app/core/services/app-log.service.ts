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
  /** Use invoke directly so a logging failure cannot recursively log itself. */
  async record(level: LogLevel, source: string, message: unknown): Promise<void> {
    if (!isTauri()) return;
    try {
      await invoke<void>("write_app_log", { level, source, message: diagnosticMessage(message) });
    } catch {
      console.warn("Eattura: unable to persist diagnostic event");
    }
  }

  async snapshot(): Promise<LogSnapshot> {
    if (!isTauri()) return { entries: [], directory: null };
    return invoke<LogSnapshot>("get_app_logs");
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
