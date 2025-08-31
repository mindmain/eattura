package fe

type TipoDocumento string

const TD01 TipoDocumento = "TD01"
const TD02 TipoDocumento = "TD02"
const TD03 TipoDocumento = "TD03"
const TD04 TipoDocumento = "TD04"
const TD05 TipoDocumento = "TD05"
const TD06 TipoDocumento = "TD06"
const TD07 TipoDocumento = "TD07"
const TD08 TipoDocumento = "TD08"
const TD10 TipoDocumento = "TD10"
const TD11 TipoDocumento = "TD11"
const TD12 TipoDocumento = "TD12"
const TD16 TipoDocumento = "TD16"
const TD17 TipoDocumento = "TD17"
const TD18 TipoDocumento = "TD18"
const TD19 TipoDocumento = "TD19"
const TD20 TipoDocumento = "TD20"
const TD21 TipoDocumento = "TD21"
const TD22 TipoDocumento = "TD22"
const TD23 TipoDocumento = "TD23"
const TD24 TipoDocumento = "TD24"
const TD25 TipoDocumento = "TD25"
const TD26 TipoDocumento = "TD26"
const TD27 TipoDocumento = "TD27"

var TipiDocumento = MapTypeString[TipoDocumento]{
	TD01: "Fattura",
	TD02: "Acconto/Anticipo su fattura",
	TD03: "Acconto/Anticipo su parcella",
	TD04: "Nota di credito",
	TD05: "Nota di debito",
	TD06: "Parcella",
	TD07: "Fattura semplificata",
	TD08: "Nota di Credito semplificata",
	TD10: "Fattura per acquisto intracomunitario beni",
	TD11: "Fattura per acquisto intracomunitario servizi",
	TD12: "Documento riepilogativo (art.6,DPR 695/1996)",
	TD16: "Integrazione fattura reverse charge interno",
	TD17: "Integrazione/autofattura per acquisto servizi dall’estero",
	TD18: "Integrazione per acquisto di beni intracomunitari",
	TD19: "Integrazione/autofattura per acquisto di beni ex art.17 c.2 DPR 633/72",
	TD20: "Autofattura per regolarizzazione e integrazione delle fatture (art.6 c.8 d.lgs. 471/97 o art.46 c.5 D.L. 331/93)",
	TD21: "Autofattura per splafonamento",
	TD22: "Estrazione beni da Deposito IVA",
	TD23: "Estrazione beni da Deposito IVA con versamento dell’IVA",
	TD24: "Fattura differita di cui all’art.21, comma 4, lett. a)",
	TD25: "Fattura differita di cui all’art.21, comma 4, terzo periodo lett. b)",
	TD26: "Cessione di beni ammortizzabili e per passaggi interni (ex art.36 DPR 633/72)",
	TD27: "Fattura per autoconsumo o per cessioni gratuite senza rivalsa",
}
