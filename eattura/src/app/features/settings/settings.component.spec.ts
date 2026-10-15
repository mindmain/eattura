import { beforeEach, describe, expect, it, vi } from "vitest";
import { TestBed } from "@angular/core/testing";
import { SettingsComponent } from "./settings.component";
import { SettingsStore } from "../../core/stores/settings.store";
import { NotificationService } from "../../core/services/notification.service";
import { SyncService } from "../../core/services/sync.service";

describe("SettingsComponent issuer identity", () => {
  const saveCompany = vi.fn().mockResolvedValue(undefined);
  const savePec = vi.fn().mockResolvedValue(undefined);
  const notifyError = vi.fn();
  let component: SettingsComponent;

  beforeEach(() => {
    vi.clearAllMocks();
    TestBed.configureTestingModule({
      providers: [
        { provide: SettingsStore, useValue: { load: vi.fn().mockResolvedValue(null), saveCompany, savePec, dataDir: () => "" } },
        { provide: NotificationService, useValue: { success: vi.fn(), error: notifyError } },
        { provide: SyncService, useValue: { getStoredConfig: () => ({ relayUrl: "", passphrase: "", deviceId: "" }) } },
      ],
    });
    component = TestBed.runInInjectionContext(() => new SettingsComponent());
    component.companyForm.patchValue({ idCodice: "12345678901", indirizzo: "Via Roma", cap: "00100", comune: "Roma" });
  });

  it("requires a company name for a business", async () => {
    expect(component.companyForm.invalid).toBe(true);
    await component.saveCompany();
    expect(saveCompany).not.toHaveBeenCalled();
  });

  it("saves first and last name without a stale business name", async () => {
    component.companyForm.patchValue({ denominazione: "Old business name", nome: "Mario", cognome: "Rossi" });
    component.onSubjectTypeChange("persona");

    await component.saveCompany();

    expect(saveCompany).toHaveBeenCalledWith(expect.objectContaining({ denominazione: null, nome: "Mario", cognome: "Rossi" }));
    expect(notifyError).not.toHaveBeenCalled();
  });

  it("requires both personal name fields", async () => {
    component.onSubjectTypeChange("persona");
    component.companyForm.patchValue({ nome: "Mario", cognome: "   " });
    await component.saveCompany();
    expect(saveCompany).not.toHaveBeenCalled();
  });

  it("restores a personal issuer when reopening settings", async () => {
    vi.mocked(TestBed.inject(SettingsStore).load).mockResolvedValueOnce({
      company: {
        denominazione: null,
        nome: "Anna",
        cognome: "Verdi",
        id_paese: "IT",
        id_codice: "12345678901",
        codice_fiscale: null,
        regime_fiscale: "RF19",
        indirizzo: "Via Roma",
        numero_civico: null,
        cap: "00100",
        comune: "Roma",
        provincia: null,
        nazione: "IT",
        telefono: null,
        email: null,
        pec: null,
      },
      pec: null,
    });

    await component.ngOnInit();

    expect(component.subjectType()).toBe("persona");
    expect(component.companyForm.controls.nome.value).toBe("Anna");
    expect(component.companyForm.controls.cognome.value).toBe("Verdi");
    expect(component.companyForm.valid).toBe(true);
  });

  it("uses the initial SDI address until the assigned PEC address is saved", async () => {
    expect(component.pecForm.controls.sdiAddress.value).toBe("sdi01@pec.fatturapa.it");
    component.pecForm.patchValue({
      pecEmail: "sender@pec.example.it",
      sdiAddress: "sdi12@pec.fatturapa.it",
    });

    await component.savePec();

    expect(savePec).toHaveBeenCalledWith(
      expect.objectContaining({
        email: "sender@pec.example.it",
        sdi_address: "sdi12@pec.fatturapa.it",
      }),
      undefined
    );
  });
});
