package error_sdi

import (
	"strings"

	"github.com/mindmain/eattura/sdi/fe"
)

const domainSdiPec = "@pec.fatturapa.it"

// checker of error Code_00330: l’indirizzo PEC indicato nel campo PECDestinatario corrisponde ad una casella PEC del SdI
func hasCode00330(invoice *fe.FatturaElettronica) bool {
	if invoice.FatturaElettronicaHeader == nil {
		return true
	}

	if invoice.FatturaElettronicaHeader.DatiTrasmissione == nil {
		return true
	}

	if invoice.FatturaElettronicaHeader.DatiTrasmissione.PECDestinatario == "" {
		return true
	}

	if strings.Contains(invoice.FatturaElettronicaHeader.DatiTrasmissione.PECDestinatario, domainSdiPec) {
		return false
	}

	return true
}

// checker of error Code_00415: DatiCassaPrevidenziale - L'elemento Ritenuta è stato valorizzato con SI, ma
// non è presente il blocco DatiRitenuta.
func hasCode00415(invoice *fe.FatturaElettronica) bool {
	if invoice.FatturaElettronicaBody == nil {
		return true
	}

	for _, body := range invoice.FatturaElettronicaBody {
		if body.DatiGenerali == nil {
			continue
		}

		if body.DatiGenerali.DatiGeneraliDocumento == nil {
			continue
		}

		if body.DatiGenerali.DatiGeneraliDocumento.DatiCassaPrevidenziale == nil {
			continue
		}

		if body.DatiGenerali.DatiGeneraliDocumento.DatiCassaPrevidenziale.Ritenuta == "SI" {
			if body.DatiGenerali.DatiGeneraliDocumento.DatiRitenuta == nil {
				return true
			}
		}
	}
	return false
}

// checker of error Code_00001: Nome file non valido
func hasCode00001(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this

	return false
}

// checker of error Code_00002: Nome file duplicato
func hasCode00002(invocie *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00404: E’ stato già trasmesso un file con identico contenuto
func hasCode00404(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00003: Le dimensioni del file superano quelle ammesse
func hasCode00003(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00102: La firma elettronica apposta al file non risulta valida
func hasCode00102(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00100: Certificato di firma scaduto
func hasCode00100(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00101: Certificato di firma revocato
func hasCode00101(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00104: La CA (Certification Authority) che ha emesso il certificato di firma non risulta nell’elenco delle CA affidabili
func hasCode00104(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00107: Il certificato di firma non è valido
func hasCode00107(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00103: Alla firma elettronica apposta al file manca il riferimento temporale
func hasCode00103(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00105: Il riferimento temporale associato alla firma elettronica apposta al file è successivo alla data di ricezione del file
func hasCode00105(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00106: Il file compresso è vuoto oppure non è leggibile
func hasCode00106(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00200: File non conforme al formato del problema: %s
func hasCode00200(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00201: Non è possibile procedere con ulteriori controlli perché gli errori di formato presenti nel file superano il numero massimo previsto (50)
func hasCode00201(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00400: A fronte di un’aliquota pari a zero, la Natura non è stata indicata o non è stata correttamente valorizzata
func hasCode00400(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00401: A fronte di un’aliquota diversa da zero, è stata indicata una Natura non compatibile con l'operazione
func hasCode00401(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00403: La data nel documento è futura al momento che SDI riceverà il messaggio
func hasCode00403(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00411: Hai valorizzato un dettaglioLinee in benieServizi con valore Ritenuta=SI ma non è presente il blocco DatiRitenuta obbligatorio
func hasCode00411(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00413: Nella cassaPrevidenziale, non è presente la natura a fronte di una Aliquota 0
func hasCode00413(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00414: Nalla cassaPrevidenziale, viene indicata la natura ma l'aliquota non è zero
func hasCode00414(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00417: Non è presente un identificativo fiscale PIVA o CF
func hasCode00417(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00419: DatiRiepilogo non presente in corrispondenza del blocco AliquotaIVA, per ogni AliquotaIVa deve essere presente il suo DatiRiepilogo, solo fatture ordinarie
func hasCode00419(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00420: A fronte di EsigibilitaIVA uguale a S (Split-Payment), per Natura è stato indicato il valore N6
func hasCode00420(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00424: L’aliquota non è indicata in termini percentuali
func hasCode00424(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00431: A fronte di Tipo Documento uguale a TD07 (fattura semplificata) o TD08 (nota di credito semplificata), gli Identificativi Fiscali e gli Altri Dati Identificativi del Cessionario/Committente non sono stati valorizzati
func hasCode00431(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00432: A fronte di Tipo Documento uguale a TD01 (fattura), TD04 (nota di credito) o TD05 (Nota di debito), gli Identificativi Fiscali del Cessionario/Committente non sono stati indicati
func hasCode00432(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00433: L’Imposta o l’Aliquota non sono state valorizzate
func hasCode00433(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00434: Imposta e Aliquota non coerenti
func hasCode00434(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00435: Detraibile e Deducibile non possono essere presenti contemporaneamente con riferimento agli stessi DatiRiepilogo
func hasCode00435(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00436: DataRegistrazione antecedente alla data del documento
func hasCode00436(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00442: La rettifica non è possibile perché i dati risultano già annullati
func hasCode00442(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00443: L’annullamento non è possibile perché i dati risultano già annullati
func hasCode00443(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00444: Il file originario indicato nel campo IdFile non esiste
func hasCode00444(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00445: Il file indicato nel campo IdFile non è il file originario
func hasCode00445(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00446: Posizione non trovata all’interno del file originario
func hasCode00446(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00447: Un file di rettifica non deve contenere più di un documento
func hasCode00447(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00448: (controllo in vigore dal primo ottobre 2020) non è più ammesso il valore generico N2, N3 o N6 come codice natura dell’operazione
func hasCode00448(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00460: Il Tipo Documento non è coerente con il Paese del Cedente/Prestatore
func hasCode00460(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00461: Il Tipo Documento non è ammesso per le fatture emesse
func hasCode00461(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00462: La Data deve essere valorizzata
func hasCode00462(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00464: Gli Identificativi Fiscali del Cedente/Prestatore non sono stati valorizzati
func hasCode00464(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00467: Con riferimento allo stesso blocco Cessionario/Committente, sono stati riportati i dati di documenti riepilogativi insieme ad altri tipi di documento
func hasCode00467(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00468: Con riferimento allo stesso blocco Cedente/Prestatore, sono stati riportati i dati di documenti riepilogativi insieme ad altri tipi di documento
func hasCode00468(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00469: Il soggetto che ha firmato il file non corrisponde né al firmatario del file originario né al soggetto titolare dei dati contenuti nel file originario
func hasCode00469(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00470: E’ stata modificata la partita IVA e/o il codice fiscale del soggetto titolare dei dati contenuti nel file originario
func hasCode00470(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00301: La partita IVA del Cedente/Prestatore non è valida
func hasCode00301(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00302: Il Codice Fiscale del Cedente/Prestatore non è valido
func hasCode00302(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00303: La partita IVA del Rappresentante Fiscale non è valida
func hasCode00303(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00305: La partita IVA del Cessionario/Committente non è valida
func hasCode00305(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00306: Il Codice Fiscale del Cessionario/Committente non è valido
func hasCode00306(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00320: Il codice fiscale, non fa parte del gruppo IVA indicato
func hasCode00320(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00321: In presenza di una partita IVA di gruppo IVA del Cedente/Prestatore occorre valorizzare il Codice Fiscale del Cedente/Prestatore con quello del soggetto partecipante al gruppo
func hasCode00321(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00322: Il codice fiscale è presente ma non è partecipe al gruppo IVA
func hasCode00322(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00325: In presenza di una partita IVA di gruppo IVA del Cessionario/Committente occorre valorizzare il Codice Fiscale del Cessionario/Committente con quello del soggetto partecipante al gruppo
func hasCode00325(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00600: Soggetto non autorizzato alla trasmissione
func hasCode00600(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00500: Partita IVA del Cedente/Prestatore cessata in Anagrafe Tributaria
func hasCode00500(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00501: Partita IVA del Cessionario/Committente cessata in Anagrafe Tributaria
func hasCode00501(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00502: Partita IVA del Rappresentante Fiscale cessata in Anagrafe Tributaria
func hasCode00502(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00503: La data del documento non è compatibile con il periodo di riferimento
func hasCode00503(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}

// checker of error Code_00504: La data di registrazione del documento non è compatibile con il periodo di riferimento
func hasCode00504(invoice *fe.FatturaElettronica) bool {
	// TODO: Implement this
	return false
}
