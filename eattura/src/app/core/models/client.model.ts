export interface ClientSummary {
  id: string;
  denominazione: string | null;
  nome: string | null;
  cognome: string | null;
  idPaese: string;
  idCodice: string;
  comune: string;
}

export interface ClientDetail extends ClientSummary {
  codiceFiscale: string | null;
  regimeFiscale: string | null;
  indirizzo: string;
  numeroCivico: string | null;
  cap: string;
  provincia: string | null;
  nazione: string;
  telefono: string | null;
  email: string | null;
  pec: string | null;
  codiceDestinatario: string | null;
}

export interface CreateClientRequest {
  denominazione: string | null;
  nome: string | null;
  cognome: string | null;
  idPaese: string;
  idCodice: string;
  codiceFiscale: string | null;
  regimeFiscale: string | null;
  indirizzo: string;
  numeroCivico: string | null;
  cap: string;
  comune: string;
  provincia: string | null;
  nazione: string;
  telefono: string | null;
  email: string | null;
  pec: string | null;
  codiceDestinatario: string | null;
}
