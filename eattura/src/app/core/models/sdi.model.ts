/** SDI document type codes and their Italian descriptions. */
export const TIPO_DOCUMENTO: Record<string, string> = {
  TD01: "Fattura",
  TD02: "Acconto/anticipo su fattura",
  TD03: "Acconto/anticipo su parcella",
  TD04: "Nota di credito",
  TD05: "Nota di debito",
  TD06: "Parcella",
  TD16: "Integrazione fattura reverse charge interno",
  TD17: "Integrazione/autofattura per acquisto servizi dall'estero",
  TD18: "Integrazione per acquisto beni intracomunitari",
  TD19: "Integrazione/autofattura per acquisto beni art. 17 c.2 DPR 633/72",
  TD20: "Autofattura per regolarizzazione e integrazione",
  TD21: "Autofattura per splafonamento",
  TD22: "Estrazione beni da deposito IVA",
  TD23: "Estrazione beni da deposito IVA con pagamento IVA",
  TD24: "Fattura differita (art. 21 c.4 lett. a)",
  TD25: "Fattura differita (art. 21 c.4 terzo periodo lett. b)",
  TD26: "Cessione beni ammortizzabili e passaggi interni",
  TD27: "Fattura per autoconsumo o cessioni gratuite senza rivalsa",
  TD28: "Acquisti da San Marino con IVA",
};

/** SDI tax regime codes and their Italian descriptions. */
export const REGIME_FISCALE: Record<string, string> = {
  RF01: "Regime ordinario",
  RF02: "Regime dei contribuenti minimi",
  RF04: "Agricoltura e attività connesse e pesca",
  RF05: "Vendita sali e tabacchi",
  RF06: "Commercio fiammiferi",
  RF07: "Editoria",
  RF08: "Gestione servizi telefonia pubblica",
  RF09: "Rivendita documenti di trasporto pubblico e sosta",
  RF10: "Intrattenimenti, giochi e altre attività (art. 74 c.6)",
  RF11: "Agenzie di viaggi e turismo",
  RF12: "Agriturismo",
  RF13: "Vendite a domicilio",
  RF14: "Rivendita beni usati, oggetti d'arte, antiquariato, collezione",
  RF15: "Agenzie di vendite all'asta di oggetti d'arte, antiquariato, collezione",
  RF16: "IVA per cassa P.A.",
  RF17: "IVA per cassa (art. 32-bis DL 83/2012)",
  RF18: "Altro",
  RF19: "Regime forfettario",
};

/** SDI payment method codes and their Italian descriptions. */
export const MODALITA_PAGAMENTO: Record<string, string> = {
  MP01: "Contanti",
  MP02: "Assegno",
  MP03: "Assegno circolare",
  MP04: "Contanti presso Tesoreria",
  MP05: "Bonifico",
  MP06: "Vaglia cambiario",
  MP07: "Bollettino bancario",
  MP08: "Carta di pagamento",
  MP09: "RID",
  MP10: "RID utenze",
  MP11: "RID veloce",
  MP12: "RIBA",
  MP13: "MAV",
  MP14: "Quietanza erario",
  MP15: "Giroconto su conti di contabilità speciale",
  MP16: "Domiciliazione bancaria",
  MP17: "Domiciliazione postale",
  MP18: "Bollettino di c/c postale",
  MP19: "SEPA Direct Debit",
  MP20: "SEPA Direct Debit CORE",
  MP21: "SEPA Direct Debit B2B",
  MP22: "Trattenuta su somme già riscosse",
  MP23: "PagoPA",
};

/** SDI payment condition codes. */
export const CONDIZIONI_PAGAMENTO: Record<string, string> = {
  TP01: "Pagamento a rate",
  TP02: "Pagamento completo",
  TP03: "Anticipo",
};

/** SDI VAT nature codes for zero-rate operations. */
export const NATURA: Record<string, string> = {
  N1: "Escluse ex art. 15",
  N2: "Non soggette",
  "N2.1": "Non soggette - artt. da 7 a 7-septies DPR 633/72",
  "N2.2": "Non soggette - altri casi",
  N3: "Non imponibili",
  "N3.1": "Non imponibili - esportazioni",
  "N3.2": "Non imponibili - cessioni intracomunitarie",
  "N3.3": "Non imponibili - cessioni verso San Marino",
  "N3.4": "Non imponibili - operazioni assimilate alle cessioni all'esportazione",
  "N3.5": "Non imponibili - a seguito di dichiarazioni d'intento",
  "N3.6": "Non imponibili - altre operazioni non concorrenti alla formazione del plafond",
  N4: "Esenti",
  N5: "Regime del margine / IVA non esposta in fattura",
  N6: "Inversione contabile (reverse charge)",
  "N6.1": "Inversione contabile - cessione rottami e materiali di recupero",
  "N6.2": "Inversione contabile - cessione oro e argento puro",
  "N6.3": "Inversione contabile - subappalto nel settore edile",
  "N6.4": "Inversione contabile - cessione fabbricati",
  "N6.5": "Inversione contabile - cessione telefoni cellulari",
  "N6.6": "Inversione contabile - cessione prodotti elettronici",
  "N6.7": "Inversione contabile - prestazioni comparto edile e settori connessi",
  "N6.8": "Inversione contabile - operazioni settore energetico",
  "N6.9": "Inversione contabile - altri casi",
  N7: "IVA assolta in altro stato UE",
};

/** SDI VAT exigibility types. */
export const ESIGIBILITA_IVA: Record<string, string> = {
  D: "Esigibilità differita",
  I: "Esigibilità immediata",
  S: "Scissione dei pagamenti",
};

/** SDI professional pension fund types (Casse di Previdenza TC01-TC25). */
export const TIPO_CASSA: Record<string, string> = {
  TC01: "TC01 - Cassa Nazionale Previdenza e Assistenza Avvocati e Procuratori Legali",
  TC02: "TC02 - Cassa Nazionale Previdenza e Assistenza Dottori Commercialisti",
  TC03: "TC03 - Cassa Nazionale Previdenza e Assistenza Geometri",
  TC04: "TC04 - Cassa Nazionale Previdenza e Assistenza Ingegneri e Architetti (Inarcassa)",
  TC05: "TC05 - Cassa Nazionale Notariato",
  TC06: "TC06 - Cassa Nazionale Previdenza e Assistenza Ragionieri e Periti Commerciali",
  TC07: "TC07 - Ente Nazionale Previdenza e Assistenza Periti Industriali (EPPI)",
  TC08: "TC08 - Ente Nazionale Previdenza e Assistenza Biologi (ENPAB)",
  TC09: "TC09 - Ente Nazionale Previdenza e Assistenza Professione Infermieristica (ENPAPI)",
  TC10: "TC10 - Ente Nazionale Previdenza e Assistenza Psicologi (ENPAP)",
  TC11: "TC11 - Ente Nazionale Previdenza e Assistenza Veterinari (ENPAV)",
  TC12: "TC12 - Ente Nazionale Previdenza e Assistenza Consulenti del Lavoro (ENPACL)",
  TC13: "TC13 - Cassa Previdenza e Assistenza Dottori Agronomi e Forestali",
  TC14: "TC14 - Cassa Nazionale Previdenza e Assistenza Giornalisti Italiani",
  TC15: "TC15 - Cassa Nazionale Previdenza e Assistenza Spedizionieri Doganali",
  TC16: "TC16 - Gestione Separata INPS ex art. 2 c. 26 L. 335/95",
  TC17: "TC17 - Cassa Pensioni Medici (ENPAM)",
  TC18: "TC18 - Cassa Pensioni Farmacisti (ENPAF)",
  TC19: "TC19 - Cassa Pensioni Agenti e Rappresentanti di Commercio (ENASARCO)",
  TC20: "TC20 - Cassa Pensioni periti agrari e periti agrari laureati (ENPAIA)",
  TC21: "TC21 - Fondo previdenza impiegati agricoli e forestali (ENPAIA)",
  TC22: "TC22 - Gestione Autonoma INPS (Rivalsa INPS 4%)",
};

/** SDI withholding tax types (RT01-RT06). */
export const TIPO_RITENUTA: Record<string, string> = {
  RT01: "RT01 - Ritenuta persone fisiche (es. professionisti)",
  RT02: "RT02 - Ritenuta persone giuridiche",
  RT03: "RT03 - Contributo INPS",
  RT04: "RT04 - Contributo ENASARCO",
  RT05: "RT05 - Contributo ENPAM",
  RT06: "RT06 - Altro contributo previdenziale",
};

/** SDI payment reason codes for withholding tax. */
export const CAUSALE_PAGAMENTO_RITENUTA: Record<string, string> = {
  A: "A - Prestazioni di lavoro autonomo (arti e professioni abituali)",
  M: "M - Prestazioni di lavoro autonomo non esercitate abitualmente (occasionali)",
  O: "O - Prestazioni di lavoro autonomo non abituali con obbligo previdenziale",
  Q: "Q - Provvigioni corrisposte ad agente monomandatario",
  R: "R - Provvigioni corrisposte ad agente plurimandatario",
  V: "V - Provvigioni corrisposte a mediatore",
};

/** Documentary guidance for SDI VAT nature codes (specifiche tecniche FatturaPA v1.9). */
export const NATURA_DETAILS: Record<string, string> = {
  N1: "Anticipazioni in nome e per conto del cliente, interessi moratori o penalità esclusi dalla base imponibile (art. 15 DPR 633/72).",
  "N2.1": "Operazioni carenti del requisito territoriale IVA (servizi/cessioni extra-UE o UE ex artt. da 7 a 7-septies DPR 633/72).",
  "N2.2": "Operazioni non soggette ad IVA per regimi agevolati: Regime Forfettario (L. 190/2014), Regime di vantaggio/minimi o altri casi.",
  "N3.1": "Cessioni all'esportazione diretta o triangolare fuori dall'Unione Europea (art. 8 c. 1 lett. a/b DPR 633/72).",
  "N3.2": "Cessioni intracomunitarie di beni verso operatori economici iscritti al VIES (art. 41 DL 331/93).",
  "N3.3": "Cessioni di beni verso operatori della Repubblica di San Marino (art. 71 DPR 633/72).",
  "N3.4": "Operazioni assimilate alle cessioni all'esportazione e servizi internazionali (artt. 8-bis e 9 DPR 633/72).",
  "N3.5": "Operazioni non imponibili verso esportatori abituali a seguito di lettera d'intento (art. 8 c. 1 lett. c DPR 633/72).",
  "N3.6": "Altre operazioni non imponibili che non concorrono alla formazione del plafond IVA.",
  N4: "Operazioni esenti da IVA (prestazioni sanitarie, didattiche, assicurative o finanziarie ex art. 10 DPR 633/72).",
  N5: "Beni usati, oggetti d'arte, editoria o agenzie di viaggio in regime del margine con IVA non esposta.",
  "N6.1": "Inversione contabile (reverse charge) per cessione di rottami, cascami e materiali di recupero (art. 74 c. 7-8).",
  "N6.2": "Inversione contabile per cessione di oro da investimento o argento puro (art. 17 c. 5).",
  "N6.3": "Inversione contabile per prestazioni di servizi rese da subappaltatori nel settore edile (art. 17 c. 6 lett. a).",
  "N6.4": "Inversione contabile per cessione di fabbricati o porzioni di fabbricato con opzione IVA (art. 17 c. 6 lett. a-bis).",
  "N6.5": "Inversione contabile per cessione di telefoni cellulari (art. 17 c. 6 lett. b).",
  "N6.6": "Inversione contabile per cessione di console da gioco, tablet PC, laptop e circuiti integrati (art. 17 c. 6 lett. c).",
  "N6.7": "Inversione contabile per prestazioni di pulizia, demolizione, installazione impianti e completamento edifici (art. 17 c. 6 lett. a-ter).",
  "N6.8": "Inversione contabile per operazioni del settore energetico, gas ed emissioni (art. 17 c. 6 lett. d-bis/ter/quater).",
  "N6.9": "Inversione contabile per altri casi previsti dalla normativa vigente.",
  N7: "Vendite a distanza intra-UE (OSS) o servizi TTE con IVA assolta in altro Stato membro UE.",
};

/** Selectable VAT nature codes allowed on new FatturaPA v1.9 invoices (excludes deprecated generic N2, N3, N6). */
export const SELECTABLE_NATURA_CODES: readonly string[] = [
  "N1",
  "N2.1",
  "N2.2",
  "N3.1",
  "N3.2",
  "N3.3",
  "N3.4",
  "N3.5",
  "N3.6",
  "N4",
  "N5",
  "N6.1",
  "N6.2",
  "N6.3",
  "N6.4",
  "N6.5",
  "N6.6",
  "N6.7",
  "N6.8",
  "N6.9",
  "N7",
];

/** Nature codes subject to €2.00 virtual stamp duty when total exceeds €77.47 (D.M. 17/06/2014). */
export const BOLLO_SUBJECT_NATURA_CODES: ReadonlySet<string> = new Set([
  "N1",
  "N2",
  "N2.1",
  "N2.2",
  "N3.5",
  "N3.6",
  "N4",
]);

/** Standard Italian VAT rates selectable in the line editor. */
export const ALIQUOTA_IVA_PRESETS: readonly { value: number; code: string; label: string; detail: string }[] = [
  {
    value: 22,
    code: "22%",
    label: "Aliquota ordinaria",
    detail: "Aliquota IVA generale applicata alla maggior parte delle cessioni di beni e prestazioni di servizi.",
  },
  {
    value: 10,
    code: "10%",
    label: "Aliquota ridotta (10%)",
    detail: "Ristorazione, turismo, edilizia agevolata, energia e beni/servizi di cui alla Tabella A, Parte III, DPR 633/72.",
  },
  {
    value: 5,
    code: "5%",
    label: "Aliquota ridotta (5%)",
    detail: "Prestazioni socio-sanitarie/educative di cooperative, erbe aromatiche e trasporti (Tabella A, Parte II-bis).",
  },
  {
    value: 4,
    code: "4%",
    label: "Aliquota minima (4%)",
    detail: "Beni alimentari di prima necessità, libri/editoria, ausili per disabili e prima casa (Tabella A, Parte II).",
  },
  {
    value: 0,
    code: "0%",
    label: "Senza applicazione IVA (0%)",
    detail: "Operazioni non soggette (es. Forfettario N2.2), esenti (N4), escluse (N1), non imponibili (N3.x) o Reverse Charge (N6.x).",
  },
];

/** Documentary descriptions for SDI payment conditions (<CondizioniPagamento>). */
export const CONDIZIONI_PAGAMENTO_DETAILS: Record<string, string> = {
  TP01: "Elemento XML <CondizioniPagamento> = TP01 · Il corrispettivo è suddiviso in più rate di pagamento.",
  TP02: "Elemento XML <CondizioniPagamento> = TP02 · Pagamento dell'intero importo in un'unica soluzione.",
  TP03: "Elemento XML <CondizioniPagamento> = TP03 · Pagamento a titolo di anticipo o acconto.",
};

/** Documentary descriptions for SDI payment methods (<ModalitaPagamento>). */
export const MODALITA_PAGAMENTO_DETAILS: Record<string, string> = {
  MP01: "XML <ModalitaPagamento> = MP01 · Pagamento in denaro contante nei limiti di legge.",
  MP02: "XML <ModalitaPagamento> = MP02 · Pagamento tramite assegno bancario.",
  MP03: "XML <ModalitaPagamento> = MP03 · Pagamento tramite assegno circolare garantito.",
  MP04: "XML <ModalitaPagamento> = MP04 · Pagamento in contanti presso la Tesoreria dello Stato.",
  MP05: "XML <ModalitaPagamento> = MP05 · Bonifico bancario / SEPA Credit Transfer (consigliato compilare il campo IBAN).",
  MP06: "XML <ModalitaPagamento> = MP06 · Pagamento tramite vaglia cambiario.",
  MP07: "XML <ModalitaPagamento> = MP07 · Pagamento tramite bollettino bancario.",
  MP08: "XML <ModalitaPagamento> = MP08 · Carta di credito, carta di debito o POS elettronico.",
  MP09: "XML <ModalitaPagamento> = MP09 · Addebito diretto RID tradizionale.",
  MP10: "XML <ModalitaPagamento> = MP10 · Addebito diretto RID per utenze.",
  MP11: "XML <ModalitaPagamento> = MP11 · Addebito diretto RID veloce.",
  MP12: "XML <ModalitaPagamento> = MP12 · Ricevuta Bancaria elettronica (Ri.Ba.).",
  MP13: "XML <ModalitaPagamento> = MP13 · Pagamento Mediante Avviso (MAV).",
  MP14: "XML <ModalitaPagamento> = MP14 · Quietanza erario dello Stato.",
  MP15: "XML <ModalitaPagamento> = MP15 · Giroconto su conti di contabilità speciale (enti pubblici).",
  MP16: "XML <ModalitaPagamento> = MP16 · Domiciliazione bancaria su conto corrente.",
  MP17: "XML <ModalitaPagamento> = MP17 · Domiciliazione postale.",
  MP18: "XML <ModalitaPagamento> = MP18 · Bollettino di conto corrente postale.",
  MP19: "XML <ModalitaPagamento> = MP19 · Addebito diretto SEPA Direct Debit.",
  MP20: "XML <ModalitaPagamento> = MP20 · Addebito diretto SEPA Direct Debit CORE.",
  MP21: "XML <ModalitaPagamento> = MP21 · Addebito diretto SEPA Direct Debit B2B tra imprese.",
  MP22: "XML <ModalitaPagamento> = MP22 · Trattenuta su somme già riscosse.",
  MP23: "XML <ModalitaPagamento> = MP23 · Pagamento elettronico tramite piattaforma PagoPA (PA).",
};
