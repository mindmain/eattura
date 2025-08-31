package fe

type MetodoPagamento string

const MP01 MetodoPagamento = "MP01"
const MP02 MetodoPagamento = "MP02"
const MP03 MetodoPagamento = "MP03"
const MP04 MetodoPagamento = "MP04"
const MP05 MetodoPagamento = "MP05"
const MP06 MetodoPagamento = "MP06"
const MP07 MetodoPagamento = "MP07"
const MP08 MetodoPagamento = "MP08"
const MP09 MetodoPagamento = "MP09"
const MP10 MetodoPagamento = "MP10"
const MP11 MetodoPagamento = "MP11"
const MP12 MetodoPagamento = "MP12"
const MP13 MetodoPagamento = "MP13"
const MP14 MetodoPagamento = "MP14"
const MP15 MetodoPagamento = "MP15"
const MP16 MetodoPagamento = "MP16"
const MP17 MetodoPagamento = "MP17"
const MP18 MetodoPagamento = "MP18"
const MP19 MetodoPagamento = "MP19"
const MP20 MetodoPagamento = "MP20"
const MP21 MetodoPagamento = "MP21"
const MP22 MetodoPagamento = "MP22"
const MP23 MetodoPagamento = "MP23"

var MetodiPagamento = MapTypeString[MetodoPagamento]{
	MP01: "contanti",
	MP02: "assegno",
	MP03: "assegno circolare",
	MP04: "contanti presso Tesoreria",
	MP05: "bonifico",
	MP06: "vaglia cambiario",
	MP07: "bollettino bancario",
	MP08: "carta di pagamento",
	MP09: "RID",
	MP10: "RID utenze",
	MP11: "RID veloce",
	MP12: "Riba",
	MP13: "MAV",
	MP14: "quietanza erario stato",
	MP15: "giroconto su conti di contabilità speciale",
	MP16: "domiciliazione bancaria",
	MP17: "domiciliazione postale",
	MP18: "bollettino di c/c postale",
	MP19: "SEPA Direct Debit",
	MP20: "SEPA Direct Debit CORE",
	MP21: "SEPA Direct Debit B2B",
	MP22: "Trattenuta su somme già riscosse",
	MP23: "PagoPA",
}
