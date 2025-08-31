package fe

type TipoCessionePrestazione string

const SC TipoCessionePrestazione = "SC"
const PR TipoCessionePrestazione = "PR"
const AB TipoCessionePrestazione = "AB"
const AC TipoCessionePrestazione = "AC"

var TipiCessionePrestazione = MapTypeString[TipoCessionePrestazione]{
	"SC": "Sconto",
	"PR": "Premio",
	"AB": "Abbuono",
	"AC": "Spesa accessoria",
}
