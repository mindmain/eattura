package fe

type CondizionePagamento string

const TP01 CondizionePagamento = "TP01"
const TP02 CondizionePagamento = "TP02"
const TP03 CondizionePagamento = "TP03"

var CondizioniPagamento = MapTypeString[CondizionePagamento]{
	TP01: "pagamento a rate",
	TP02: "pagamento completo",
	TP03: "anticipo",
}
