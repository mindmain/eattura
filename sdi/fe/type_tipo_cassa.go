package fe

type TipoCassa string

const (
	TC01 TipoCassa = "TC01"
	TC02 TipoCassa = "TC02"
	TC03 TipoCassa = "TC03"
	TC04 TipoCassa = "TC04"
	TC05 TipoCassa = "TC05"
	TC06 TipoCassa = "TC06"
	TC07 TipoCassa = "TC07"
	TC08 TipoCassa = "TC08"
	TC09 TipoCassa = "TC09"
	TC10 TipoCassa = "TC10"
	TC11 TipoCassa = "TC11"
	TC12 TipoCassa = "TC12"
	TC13 TipoCassa = "TC13"
	TC14 TipoCassa = "TC14"
	TC15 TipoCassa = "TC15"
	TC16 TipoCassa = "TC16"
	TC17 TipoCassa = "TC17"
	TC18 TipoCassa = "TC18"
	TC19 TipoCassa = "TC19"
	TC20 TipoCassa = "TC20"
	TC21 TipoCassa = "TC21"
	TC22 TipoCassa = "TC22"
)

var TipiCassa = MapTypeString[TipoCassa]{

	TC01: "Cassa Nazionale Previdenza e Assistenza Avvocati e Procuratori Legali",
	TC02: "Cassa Previdenza Dottori Commercialisti",
	TC03: "Cassa Previdenza e Assistenza Geometri",
	TC04: "Cassa Nazionale Previdenza e Assistenza Ingegneri e Architetti Liberi Professionisti",
	TC05: "Cassa Nazionale del Notariato",
	TC06: "Cassa Nazionale Previdenza e Assistenza Ragionieri e Periti Commerciali",
	TC07: "Ente Nazionale Assistenza Agenti e Rappresentanti di Commercio (ENASARCO)",
	TC08: "Ente Nazionale Previdenza e Assistenza Consulenti del Lavoro (ENPACL)",
	TC09: "Ente Nazionale Previdenza e Assistenza Medici (ENPAM)",
	TC10: "Ente Nazionale Previdenza e Assistenza Farmacisti (ENPAF)",
	TC11: "Ente Nazionale Previdenza e Assistenza Veterinari (ENPAV)",
	TC12: "Ente Nazionale Previdenza e Assistenza Impiegati dell'Agricoltura (ENPAIA)",
	TC13: "Fondo Previdenza Impiegati Imprese di Spedizione e Agenzie Marittime",
	TC14: "Istituto Nazionale Previdenza Giornalisti Italiani (INPGI)",
	TC15: "Opera Nazionale Assistenza Orfani Sanitari Italiani (ONAOSI)",
	TC16: "Autonoma Assistenza Integrativa Giornalisti Italiani (CASAGIT)",
	TC17: "Ente Previdenza Periti Industriali e Periti Industriali Laureati (EPPI)",
	TC18: "Ente Previdenza e Assistenza Pluricategoriale (EPAP)",
	TC19: "Ente Nazionale Previdenza e Assistenza Biologi (ENPAB)",
	TC20: "Ente Nazionale Previdenza e Assistenza Professione Infermieristica (ENPAPI)",
	TC21: "Ente Nazionale Previdenzae Assistenza Psicologi (ENPAP)",
	TC22: "INPS",
}
