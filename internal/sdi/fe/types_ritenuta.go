package fe

type TipoRitenuta string

const RT01 TipoRitenuta = "RT01"
const RT02 TipoRitenuta = "RT02"
const RT03 TipoRitenuta = "RT03"
const RT04 TipoRitenuta = "RT04"
const RT05 TipoRitenuta = "RT05"
const RT06 TipoRitenuta = "RT06"

var TipiRitenuta = MapTypeString[TipoRitenuta]{
	RT01: "Ritenuta persone fisiche",
	RT02: "Ritenuta persone giuridiche",
	RT03: "Contributo INPS",
	RT04: "Contributo ENASARCO",
	RT05: "Contributo ENPAM",
	RT06: "Altro contributo previdenziale",
}
