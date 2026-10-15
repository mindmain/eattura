import { describe, it, expect, beforeEach } from "vitest";
import { TestBed } from "@angular/core/testing";
import { ViesService } from "./vies.service";
import { TauriService } from "./tauri.service";

describe("ViesService", () => {
  let service: ViesService;
  let tauriSpy: { invoke: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    tauriSpy = {
      invoke: vi.fn(),
    };

    TestBed.configureTestingModule({
      providers: [
        ViesService,
        { provide: TauriService, useValue: tauriSpy },
      ],
    });
    service = TestBed.inject(ViesService);
  });

  it("should be created", () => {
    expect(service).toBeDefined();
  });

  it("should call lookup_vat_vies via Tauri IPC", async () => {
    tauriSpy.invoke.mockResolvedValueOnce({
      valid: true,
      countryCode: "IT",
      vatNumber: "01234567890",
      name: "ACME SRL",
      address: "VIA ROMA 10, 00100 ROMA RM",
      street: "VIA ROMA 10",
      postalCode: "00100",
      city: "ROMA",
      province: "RM",
    });

    const res = await service.lookupVat("IT", "01234567890");
    expect(tauriSpy.invoke).toHaveBeenCalledWith("lookup_vat_vies", {
      countryCode: "IT",
      vatNumber: "01234567890",
    });
    expect(res.valid).toBe(true);
    expect(res.name).toBe("ACME SRL");
    expect(res.city).toBe("ROMA");
  });
});
