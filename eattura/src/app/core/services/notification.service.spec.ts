import { describe, it, expect, beforeEach, vi } from "vitest";
import { NotificationService } from "./notification.service";

describe("NotificationService", () => {
  let service: NotificationService;

  beforeEach(() => {
    vi.useFakeTimers();
    service = new NotificationService();
  });

  it("success() aggiunge una notifica con tipo success", () => {
    service.success("Operazione riuscita");
    const notifications = service.notifications();

    expect(notifications).toHaveLength(1);
    expect(notifications[0].message).toBe("Operazione riuscita");
    expect(notifications[0].type).toBe("success");
  });

  it("error() aggiunge una notifica con tipo error", () => {
    service.error("Errore imprevisto");
    const notifications = service.notifications();

    expect(notifications).toHaveLength(1);
    expect(notifications[0].message).toBe("Errore imprevisto");
    expect(notifications[0].type).toBe("error");
  });

  it("dismiss() rimuove la notifica con l'id specificato", () => {
    service.success("Msg 1");
    service.error("Msg 2");
    expect(service.notifications()).toHaveLength(2);

    const firstId = service.notifications()[0].id;
    service.dismiss(firstId);

    expect(service.notifications()).toHaveLength(1);
    expect(service.notifications()[0].message).toBe("Msg 2");
  });

  it("auto-dismiss rimuove la notifica dopo 5 secondi", () => {
    service.success("Temporanea");
    expect(service.notifications()).toHaveLength(1);

    vi.advanceTimersByTime(5000);

    expect(service.notifications()).toHaveLength(0);
  });
});
