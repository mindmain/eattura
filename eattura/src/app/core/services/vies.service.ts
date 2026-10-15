import { inject, Injectable } from "@angular/core";
import { TauriService } from "./tauri.service";
import type { VatLookupResponse } from "../models/vies.model";

/**
 * Service for stateless VAT lookup and validation via VIES.
 * Invokes the Tauri backend command or falls back to the passacarte proxy endpoint.
 */
@Injectable({ providedIn: "root" })
export class ViesService {
  private tauri = inject(TauriService);

  async lookupVat(countryCode: string, vatNumber: string): Promise<VatLookupResponse> {
    try {
      return await this.tauri.invoke<VatLookupResponse>("lookup_vat_vies", {
        countryCode,
        vatNumber,
      });
    } catch (err: unknown) {
      const isBrowser = typeof window !== "undefined" && !(window as any).__TAURI_INTERNALS__;
      if (isBrowser) {
        try {
          const res = await fetch(
            `/api/proxy/vat-lookup?country_code=${encodeURIComponent(countryCode)}&vat_number=${encodeURIComponent(vatNumber)}`
          );
          if (res.ok) {
            return (await res.json()) as VatLookupResponse;
          }
        } catch {
          // Ignore and rethrow original error
        }
      }
      throw err;
    }
  }
}
