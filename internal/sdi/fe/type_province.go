package fe

type Province string

const (
	Province_AG Province = "AG"
	Province_AL Province = "AL"
	Province_AN Province = "AN"
	Province_AO Province = "AO"
	Province_AR Province = "AR"
	Province_AP Province = "AP"
	Province_AT Province = "AT"
	Province_AV Province = "AV"
	Province_BA Province = "BA"
	Province_BT Province = "BT"
	Province_BL Province = "BL"
	Province_BN Province = "BN"
	Province_BG Province = "BG"
	Province_BI Province = "BI"
	Province_BO Province = "BO"
	Province_BZ Province = "BZ"
	Province_BS Province = "BS"
	Province_BR Province = "BR"
	Province_CA Province = "CA"
	Province_CL Province = "CL"
	Province_CB Province = "CB"
	Province_CI Province = "CI"
	Province_CE Province = "CE"
	Province_CT Province = "CT"
	Province_CZ Province = "CZ"
	Province_CH Province = "CH"
	Province_CO Province = "CO"
	Province_CS Province = "CS"
	Province_CR Province = "CR"
	Province_KR Province = "KR"
	Province_CN Province = "CN"
	Province_EN Province = "EN"
	Province_FM Province = "FM"
	Province_FE Province = "FE"
	Province_FI Province = "FI"
	Province_FG Province = "FG"
	Province_FC Province = "FC"
	Province_FR Province = "FR"
	Province_GE Province = "GE"
	Province_GO Province = "GO"
	Province_GR Province = "GR"
	Province_IM Province = "IM"
	Province_IS Province = "IS"
	Province_SP Province = "SP"
	Province_AQ Province = "AQ"
	Province_LT Province = "LT"
	Province_LE Province = "LE"
	Province_LC Province = "LC"
	Province_LI Province = "LI"
	Province_LO Province = "LO"
	Province_LU Province = "LU"
	Province_MC Province = "MC"
	Province_MN Province = "MN"
	Province_MS Province = "MS"
	Province_MT Province = "MT"
	Province_ME Province = "ME"
	Province_MI Province = "MI"
	Province_MO Province = "MO"
	Province_MB Province = "MB"
	Province_NA Province = "NA"
	Province_NO Province = "NO"
	Province_NU Province = "NU"
	Province_OT Province = "OT"
	Province_OR Province = "OR"
	Province_PD Province = "PD"
	Province_PA Province = "PA"
	Province_PR Province = "PR"
	Province_PV Province = "PV"
	Province_PG Province = "PG"
	Province_PU Province = "PU"
	Province_PE Province = "PE"
	Province_PC Province = "PC"
	Province_PI Province = "PI"
	Province_PT Province = "PT"
	Province_PN Province = "PN"
	Province_PZ Province = "PZ"
	Province_PO Province = "PO"
	Province_RG Province = "RG"
	Province_RA Province = "RA"
	Province_RC Province = "RC"
	Province_RE Province = "RE"
	Province_RI Province = "RI"
	Province_RN Province = "RN"
	Province_RM Province = "RM"
	Province_RO Province = "RO"
	Province_SA Province = "SA"
	Province_VS Province = "VS"
	Province_SS Province = "SS"
	Province_SV Province = "SV"
	Province_SI Province = "SI"
	Province_SR Province = "SR"
	Province_SO Province = "SO"
	Province_TA Province = "TA"
	Province_TE Province = "TE"
	Province_TR Province = "TR"
	Province_TO Province = "TO"
	Province_OG Province = "OG"
	Province_TP Province = "TP"
	Province_TN Province = "TN"
	Province_TV Province = "TV"
	Province_TS Province = "TS"
	Province_UD Province = "UD"
	Province_VA Province = "VA"
	Province_VE Province = "VE"
	Province_VB Province = "VB"
	Province_VC Province = "VC"
	Province_VR Province = "VR"
	Province_VV Province = "VV"
	Province_VI Province = "VI"
	Province_VT Province = "VT"
)

var ProvinceMap = map[Province]string{
	Province_AG: "Agrigento",
	Province_AL: "Alessandria",
	Province_AN: "Ancona",
	Province_AO: "Aosta",
	Province_AR: "Arezzo",
	Province_AP: "Ascoli Piceno",
	Province_AT: "Asti",
	Province_AV: "Avellino",
	Province_BA: "Bari",
	Province_BT: "Barletta-Andria-Trani",
	Province_BL: "Belluno",
	Province_BN: "Benevento",
	Province_BG: "Bergamo",
	Province_BI: "Biella",
	Province_BO: "Bologna",
	Province_BZ: "Bolzano",
	Province_BS: "Brescia",
	Province_BR: "Brindisi",
	Province_CA: "Cagliari",
	Province_CL: "Caltanissetta",
	Province_CB: "Campobasso",
	Province_CI: "Carbonia-Iglesias",
	Province_CE: "Caserta",
	Province_CT: "Catania",
	Province_CZ: "Catanzaro",
	Province_CH: "Chieti",
	Province_CO: "Como",
	Province_CS: "Cosenza",
	Province_CR: "Cremona",
	Province_KR: "Crotone",
	Province_CN: "Cuneo",
	Province_EN: "Enna",
	Province_FM: "Fermo",
	Province_FE: "Ferrara",
	Province_FI: "Firenze",
	Province_FG: "Foggia",
	Province_FC: "Forlì-Cesena",
	Province_FR: "Frosinone",
	Province_GE: "Genova",
	Province_GO: "Gorizia",
	Province_GR: "Grosseto",
	Province_IM: "Imperia",
	Province_IS: "Isernia",
	Province_SP: "La Spezia",
	Province_AQ: "L'Aquila",
	Province_LT: "Latina",
	Province_LE: "Lecce",
	Province_LC: "Lecco",
	Province_LI: "Livorno",
	Province_LO: "Lodi",
	Province_LU: "Lucca",
	Province_MC: "Macerata",
	Province_MN: "Mantova",
	Province_MS: "Massa-Carrara",
	Province_MT: "Matera",
	Province_ME: "Messina",
	Province_MI: "Milano",
	Province_MO: "Modena",
	Province_MB: "Monza e Brianza",
	Province_NA: "Napoli",
	Province_NO: "Novara",
	Province_NU: "Nuoro",
	Province_OT: "Olbia-Tempio",
	Province_OR: "Oristano",
	Province_PD: "Padova",
	Province_PA: "Palermo",
	Province_PR: "Parma",
	Province_PV: "Pavia",
	Province_PG: "Perugia",
	Province_PU: "Pesaro e Urbino",
	Province_PE: "Pescara",
	Province_PC: "Piacenza",
	Province_PI: "Pisa",
	Province_PT: "Pistoia",
	Province_PN: "Pordenone",
	Province_PZ: "Potenza",
	Province_PO: "Prato",
	Province_RG: "Ragusa",
	Province_RA: "Ravenna",
	Province_RC: "Reggio Calabria",
	Province_RE: "Reggio Emilia",
	Province_RI: "Rieti",
	Province_RN: "Rimini",
	Province_RM: "Roma",
	Province_RO: "Rovigo",
	Province_SA: "Salerno",
	Province_VS: "Medio Campidano",
	Province_SS: "Sassari",
	Province_SV: "Savona",
	Province_SI: "Siena",
	Province_SR: "Siracusa",
	Province_SO: "Sondrio",
	Province_TA: "Taranto",
	Province_TE: "Teramo",
	Province_TR: "Terni",
	Province_TO: "Torino",
	Province_OG: "Ogliastra",
	Province_TP: "Trapani",
	Province_TN: "Trento",
	Province_TV: "Treviso",
	Province_TS: "Trieste",
	Province_UD: "Udine",
	Province_VA: "Varese",
	Province_VE: "Venezia",
	Province_VB: "Verbano-Cusio-Ossola",
	Province_VC: "Vercelli",
	Province_VR: "Verona",
	Province_VV: "Vibo Valentia",
	Province_VI: "Vicenza",
	Province_VT: "Viterbo",
}
