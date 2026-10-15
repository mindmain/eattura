export interface VatLookupResponse {
  valid: boolean;
  countryCode: string;
  vatNumber: string;
  name?: string | null;
  address?: string | null;
  street?: string | null;
  postalCode?: string | null;
  city?: string | null;
  province?: string | null;
  userError?: string | null;
}
