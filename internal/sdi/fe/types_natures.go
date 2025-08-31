package fe

type Natura string

const N_1 Natura = "N1"
const N_2 Natura = "N2"
const N_2_1 Natura = "N2.1"
const N_2_2 Natura = "N2.2"
const N_3 Natura = "N3"
const N_3_1 Natura = "N3.1"
const N_3_2 Natura = "N3.2"
const N_3_3 Natura = "N3.3"
const N_3_4 Natura = "N3.4"
const N_3_5 Natura = "N3.5"
const N_3_6 Natura = "N3.6"
const N_4 Natura = "N4"
const N_5 Natura = "N5"
const N_6 Natura = "N6"
const N_6_1 Natura = "N6.1"
const N_6_2 Natura = "N6.2"
const N_6_3 Natura = "N6.3"
const N_6_4 Natura = "N6.4"
const N_6_5 Natura = "N6.5"
const N_6_6 Natura = "N6.6"
const N_6_7 Natura = "N6.7"
const N_6_8 Natura = "N6.8"
const N_6_9 Natura = "N6.9"
const N_7 Natura = "N7"

var Nature = MapTypeString[Natura]{
	N_1:   "escluse ex art.15",
	N_2:   "non soggette",
	N_2_1: "non soggette ad IVA ai sensi degli artt. Da 7 a 7-septies del DPR 633/72",
	N_2_2: "non soggette – altri casi",
	N_3:   "non imponibili",
	N_3_1: "non imponibili – esportazioni",
	N_3_2: "non imponibili – cessioni intracomunitarie",
	N_3_3: "non imponibili – cessioni verso San Marino",
	N_3_4: "non imponibili – operazioni assimilate alle cessioni all’esportazione",
	N_3_5: "non imponibili – a seguito di dichiarazioni d’intento",
	N_3_6: "non imponibili – altre operazioni che non concorrono alla formazione del plafond",
	N_4:   "esenti",
	N_5:   "regime del margine / IVA non esposta in fattura",
	N_6:   "inversione contabile (per le operazioni in reverse charge ovvero nei casi di autofatturazione per acquisti extra UE di servizi ovvero per importazioni di beni nei soli casi previsti)",
	N_6_1: "inversione contabile – cessione di rottami e altri materiali di recupero",
	N_6_2: "inversione contabile – cessione di oro e argento puro",
	N_6_3: "inversione contabile – subappalto nel settore edile",
	N_6_4: "inversione contabile – cessione di fabbricati",
	N_6_5: "inversione contabile – cessione di telefoni cellulari",
	N_6_6: "inversione contabile – cessione di prodotti elettronici",
	N_6_7: "inversione contabile – prestazioni comparto edile e settori connessi",
	N_6_8: "inversione contabile – operazioni settore energetico",
	N_6_9: "inversione contabile – altri casi",
	N_7:   "IVA assolta in altro stato UE (vendite a distanza ex art. 40 commi 3 e 4 e art. 41 comma 1 lett. b, DL 331/93; prestazione di servizi di telecomunicazioni, tele-radiodiffusione ed elettronici ex art. 7-sexies lett. f, g, DPR 633/72 e art. 74-sexies, DPR 633/72)",
}
