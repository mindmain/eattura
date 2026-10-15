import { describe, it, expect, beforeEach, vi } from "vitest";
import { Injector } from "@angular/core";
import { SettingsStore } from "./settings.store";
import { TauriService } from "../services/tauri.service";
import type { AppSettings } from "./settings.store";

describe("SettingsStore", () => {
  let store: SettingsStore;
  let tauriMock: {
    invoke: ReturnType<typeof vi.fn>;
  };

  const mockSettings: AppSettings = {
    company: {
      denominazione: "Mia Azienda Srl",
      nome: null,
      cognome: null,
      id_paese: "IT",
      id_codice: "12345678901",
      codice_fiscale: null,
      regime_fiscale: "RF01",
      indirizzo: "Via Roma 10",
      numero_civico: null,
      cap: "00100",
      comune: "Roma",
      provincia: "RM",
      nazione: "IT",
      telefono: null,
      email: "info@miazienda.it",
      pec: "pec@miazienda.it",
    },
    pec: {
      email: "pec@miazienda.it",
      imap_host: "imap.pec.it",
      imap_port: 993,
      smtp_host: "smtp.pec.it",
      smtp_port: 465,
    },
  };

  beforeEach(() => {
    tauriMock = {
      invoke: vi.fn().mockImplementation((cmd: string) => {
        if (cmd === "get_settings") return Promise.resolve(mockSettings);
        if (cmd === "get_data_directory") return Promise.resolve("/data/dir");
        if (cmd === "update_settings") return Promise.resolve(mockSettings);
        if (cmd === "open_data_directory") return Promise.resolve();
        return Promise.resolve();
      }),
    };

    const injector = Injector.create({
      providers: [
        SettingsStore,
        { provide: TauriService, useValue: tauriMock },
      ],
    });

    store = injector.get(SettingsStore);
  });

  it("load() popola settings e dataDir", async () => {
    await store.load();

    expect(store.settings()).toEqual(mockSettings);
    expect(store.dataDir()).toBe("/data/dir");
    expect(store.company()?.denominazione).toBe("Mia Azienda Srl");
    expect(store.isCompanyConfigured()).toBe(true);
  });

  it("findCompanyClientId() trova l'ID del cliente corrispondente all'azienda configurata", async () => {
    await store.load();

    const clients = [
      { id: "c1", idPaese: "IT", idCodice: "99999999999" } as any,
      { id: "c2", idPaese: "IT", idCodice: "12345678901" } as any,
    ];

    const foundId = store.findCompanyClientId(clients);
    expect(foundId).toBe("c2");
  });

  it("openDataDir() invoca open_data_directory via Tauri", async () => {
    await store.openDataDir();
    expect(tauriMock.invoke).toHaveBeenCalledWith("open_data_directory");
  });
});
