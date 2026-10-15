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
