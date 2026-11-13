import type { ValidatorFn } from "@angular/forms";

const oddDigits = [1, 0, 5, 7, 9, 13, 15, 17, 19, 21];
const oddLetters = [1, 0, 5, 7, 9, 13, 15, 17, 19, 21, 2, 4, 18, 20, 11, 3, 6, 8, 12, 14, 16, 10, 22, 25, 24, 23];

export function validItalianVat(value: string): boolean {
  if (!/^[0-9]{11}$/.test(value) || value === "00000000000") return false;

  const sum = [...value].reduce((total, character, index) => {
    const digit = Number(character);
    return total + (index % 2 === 0 ? digit : (digit * 2) % 10 + Math.floor(digit * 2 / 10));
  }, 0);
  return sum % 10 === 0;
}

export function validItalianFiscalCode(value: string): boolean {
  if (validItalianVat(value)) return true;
  if (!/^[A-Z]{6}[0-9LMNPQRSTUV]{2}[ABCDEHLMPRST][0-9LMNPQRSTUV]{2}[A-Z][0-9LMNPQRSTUV]{3}[A-Z]$/.test(value)) {
    return false;
  }

  const sum = [...value.slice(0, 15)].reduce((total, character, index) => {
    const ascii = character.charCodeAt(0);
    const points = ascii < 65 ? ascii - 48 : ascii - 65;
    return total + (index % 2 === 1 ? points : ascii < 65 ? oddDigits[points] : oddLetters[points]);
  }, 0);
  return sum % 26 === value.charCodeAt(15) - 65;
}

export const italianTaxIds: ValidatorFn = (form) => {
  const country = String(form.get("idPaese")?.value ?? "").trim().toUpperCase();
  if (country !== "IT") return null;

  const vat = String(form.get("idCodice")?.value ?? "").trim();
  const fiscalCode = String(form.get("codiceFiscale")?.value ?? "").trim().toUpperCase();
  const invalidVat = vat !== "" && !validItalianVat(vat);
  const invalidFiscalCode = fiscalCode !== "" && !validItalianFiscalCode(fiscalCode);
  return invalidVat || invalidFiscalCode
    ? { ...(invalidVat && { invalidVat }), ...(invalidFiscalCode && { invalidFiscalCode }) }
    : null;
};
