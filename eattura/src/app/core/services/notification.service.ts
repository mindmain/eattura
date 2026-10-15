import { Injectable, signal } from "@angular/core";

export interface Notification {
  id: number;
  message: string;
  type: "success" | "error" | "warning";
}

/**
 * Simple notification service using signals.
 * Auto-dismisses notifications after 5 seconds.
 */
@Injectable({ providedIn: "root" })
export class NotificationService {
  readonly notifications = signal<Notification[]>([]);

  private nextId = 1;

  /** Show a success notification. */
  success(message: string): void {
    this.add(message, "success");
  }

  /** Show an error notification. */
  error(message: string): void {
    this.add(message, "error");
  }

  /** Show a warning notification. */
  warning(message: string): void {
    this.add(message, "warning");
  }

  /** Dismiss a notification by ID. */
  dismiss(id: number): void {
    this.notifications.update((list) => list.filter((n) => n.id !== id));
  }

  private add(message: string, type: "success" | "error" | "warning"): void {
    const id = this.nextId++;
    this.notifications.update((list) => [...list, { id, message, type }]);
    setTimeout(() => this.dismiss(id), 5000);
  }
}
