package fe

const HeaderXMLInvoice = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>`
const SpaceValue = "http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fatture/v1.2"

type MapTypeString[E ~string] map[E]string

type FormatoTrasmissione string

const FPR12 = FormatoTrasmissione("FPR12")
const FPA12 = FormatoTrasmissione("FPA12")

type SocioUnico string

const (
	SU = "SU"
	SM = "SM"
)

var SocioUnicoMap = MapTypeString[SocioUnico]{
	SU: "La società è a socio unico",
	SM: "La società non è a socio unico",
}

type StatoLiquidazione string

const (
	LS = "LS"
	LN = "LN"
)

var StatiLiquidazione = MapTypeString[StatoLiquidazione]{
	LS: "La società è in stato di liquidazione",
	LN: "La società non è in stato di liquidazione",
}

type SoggettoEmittente string

const (
	CC = "CC"
	TZ = "TZ"
)

var SoggettiEmittente = MapTypeString[SoggettoEmittente]{
	CC: "Cessionario/committente",
	TZ: "Soggetto terzo",
}

type ScontoMaggiorazioneType string

const (
	Sconto        ScontoMaggiorazioneType = "SC"
	Maggiorazione ScontoMaggiorazioneType = "MG"
)

var ScontiMaggiorazione = MapTypeString[ScontoMaggiorazioneType]{
	Sconto:        "Sconto",
	Maggiorazione: "Maggiorazione",
}

type EsigibilitaIVA string

const (
	Immediata EsigibilitaIVA = "I"
	Differita EsigibilitaIVA = "D"
	Scissione EsigibilitaIVA = "S"
)

var VarieEsigibilitaIVA = MapTypeString[EsigibilitaIVA]{
	Immediata: "IVA ad esigibilità immediata",
	Differita: "IVA ad esigibilità differita",
	Scissione: "scissione dei pagamenti",
}
