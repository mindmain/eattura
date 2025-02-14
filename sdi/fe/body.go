package fe

import (
	"encoding/xml"
	"fmt"
	"time"
)

type Date struct {
	time.Time
}

type F64 float64

func (f *F64) MarshalXML(e *xml.Encoder, start xml.StartElement) error {

	s := fmt.Sprintf("%.2f", *f)

	return e.EncodeElement(s, start)
}

func (f *F64) UnmarshalXML(d *xml.Decoder, start xml.StartElement) error {
	var v float64
	d.DecodeElement(&v, &start)
	*f = F64(v)
	return nil
}

func (c *Date) UnmarshalXML(d *xml.Decoder, start xml.StartElement) error {
	const shortForm = "2006-01-02" // yyyymmdd date format
	var v string
	d.DecodeElement(&v, &start)
	parse, err := time.Parse(shortForm, v)
	if err != nil {
		return err
	}
	*c = Date{parse}
	return nil
}

func (c *Date) MarshalXML(e *xml.Encoder, start xml.StartElement) error {
	const shortForm = "2006-01-02" // yyyymmdd date format

	//if date is zero, don't print it
	if c.IsZero() {
		return nil
	}

	return e.EncodeElement(c.Format(shortForm), start)
}

type Anagrafica struct {
	Denominazione string `xml:"Denominazione,omitempty" json:"Denominazione" validate:"omitempty,max=80,required_without_all=Nome Cognome"`
	Nome          string `xml:"Nome,omitempty" json:"Nome" validate:"omitempty,max=60,required_with=Cognome"`
	Cognome       string `xml:"Cognome,omitempty" json:"Cognome" validate:"omitempty,max=60,required_with=Nome"`
	Titolo        string `xml:"Titolo,omitempty" json:"Titolo" validate:"omitempty,min=2,max=10"`
	CodEORI       string `xml:"CodEORI,omitempty" json:"CodEORI" validate:"omitempty,min=13,max=17"`
}

type IdFiscaleIVA struct {
	IdPaese  string `xml:"IdPaese,omitempty" json:"IdPaese" validate:"len=2"`
	IdCodice string `xml:"IdCodice,omitempty" json:"IdCodice" validate:"max=28,isPIVA"`
}

type IndirizzoType struct {
	Indirizzo    string   `xml:"Indirizzo,omitempty" json:"Indirizzo" validate:"max=60,required"`
	NumeroCivico string   `xml:"NumeroCivico,omitempty" json:"NumeroCivico" validate:"required,max=8"`
	CAP          string   `xml:"CAP,omitempty" json:"CAP" validate:"len=5,isInteger,required"`
	Comune       string   `xml:"Comune,omitempty" json:"Comune" validate:"required,max=60"`
	Provincia    Province `xml:"Provincia,omitempty" json:"Provincia" validate:"required,len=2"`
	Nazione      string   `xml:"Nazione,omitempty" json:"Nazione" validate:"required,len=2"`
}

type DatiTrasmissione struct {
	IdTrasmittente       *IdFiscaleIVA         `xml:"IdTrasmittente,omitempty" json:"IdTrasmittente" validate:"required"`
	ProgressivoInvio     string                `xml:"ProgressivoInvio,omitempty" json:"ProgressivoInvio" validate:"required,max=10"`
	FormatoTrasmissione  FormatoTrasmissione   `xml:"FormatoTrasmissione,omitempty" json:"FormatoTrasmissione" validate:"required,oneof=FPR12 FPA12"`
	CodiceDestinatario   string                `xml:"CodiceDestinatario,omitempty" json:"CodiceDestinatario" validate:"min=6,max=7"`
	ContattiTrasmittente *ContattiTrasmittente `xml:"ContattiTrasmittente,omitempty" json:"ContattiTrasmittente"`
	PECDestinatario      string                `xml:"PECDestinatario,omitempty" json:"PECDestinatario" validate:"omitempty,min=7,max=256,email,isntSDIPec"`
}

type DatiAnagrafici struct {
	IdFiscaleIVA         *IdFiscaleIVA `xml:"IdFiscaleIVA,omitempty" json:"IdFiscaleIVA"`
	CodiceFiscale        string        `xml:"CodiceFiscale,omitempty" json:"CodiceFiscale" validate:"omitempty,min=11,max=16,isFiscalCode"`
	Anagrafica           *Anagrafica   `xml:"Anagrafica,omitempty" json:"Anagrafica"`
	AlboProfessionale    string        `xml:"AlboProfessionale,omitempty" json:"AlboProfessionale" validate:"omitempty,max=60"`
	ProvinciaAlbo        Province      `xml:"ProvinciaAlbo,omitempty" json:"ProvinciaAlbo" validate:"omitempty,len=2"`
	NumeroIscrizioneAlbo string        `xml:"NumeroIscrizioneAlbo,omitempty" json:"NumeroIscrizioneAlbo" validate:"omitempty,max=60"`
	DataIscrizioneAlbo   Date          `xml:"DataIscrizioneAlbo,omitempty" json:"DataIscrizioneAlbo"`
	RegimeFiscale        RegimeFiscale `xml:"RegimeFiscale,omitempty" json:"RegimeFiscale" validate:"omitempty,len=4,startswith=RF,isTypeRegimeFiscale"`
}

type Contatti struct {
	Telefono string `xml:"Telefono,omitempty" json:"Telefono" validate:"omitempty,min=5,max=12"`
	Fax      string `xml:"Fax,omitempty" json:"Fax"`
	Email    string `xml:"Email,omitempty" json:"Email" validate:"omitempty,email,max=255"`
}

type ContattiTrasmittente struct {
	Telefono string `xml:"Telefono,omitempty" json:"Telefono" validate:"omitempty,min=5,max=12"`
	Email    string `xml:"Email,omitempty" json:"Email" validate:"omitempty,email,max=255"`
}

type IscrizioneREA struct {
	Ufficio           string            `xml:"Ufficio,omitempty" json:"Ufficio" validate:"omitempty,max=2"`
	NumeroREA         string            `xml:"NumeroREA,omitempty" json:"NumeroREA" validate:"omitempty,max=20"`
	CapitaleSociale   F64               `xml:"CapitaleSociale,omitempty" json:"CapitaleSociale" validate:"omitempty,max=15"`
	SocioUnico        SocioUnico        `xml:"SocioUnico,omitempty" json:"SocioUnico" validate:"omitempty,max=2"`
	StatoLiquidazione StatoLiquidazione `xml:"StatoLiquidazione,omitempty" json:"StatoLiquidazione" validate:"omitempty,max=2"`
}

type CedentePrestatore struct {
	DatiAnagrafici *DatiAnagrafici `xml:"DatiAnagrafici,omitempty" json:"DatiAnagrafici"`
	Sede           *IndirizzoType  `xml:"Sede,omitempty" json:"Sede"`
	Contatti       *Contatti       `xml:"Contatti,omitempty" json:"Contatti,omitempty"`
	IscrizioneREA  *IscrizioneREA  `xml:"IscrizioneREA,omitempty" json:"IscrizioneREA,omitempty"`
}

type CessionarioCommittente struct {
	DatiAnagrafici        *DatiAnagrafici        `xml:"DatiAnagrafici,omitempty" json:"DatiAnagrafici" validate:"required"`
	Sede                  *IndirizzoType         `xml:"Sede,omitempty" json:"Sede" validate:"required"`
	StabileOrganizzazione *IndirizzoType         `xml:"StabileOrganizzazione,omitempty" json:"StabileOrganizzazione"`
	RappresentanteFiscale *RappresentanteFiscale `xml:"RappresentanteFiscale,omitempty" json:"RappresentanteFiscale"`
}

type RappresentanteFiscale struct {
	Anagrafica
	IDFiscaleIva *IdFiscaleIVA `xml:"IdFiscaleIVA,omitempty" json:"IdFiscaleIVA"`
}

type TerzoIntermediarioOSoggettoEmittente struct {
	IdFiscaleIVA  *IdFiscaleIVA `xml:"IdFiscaleIVA,omitempty" json:"IdFiscaleIVA"`
	CodiceFiscale string        ` xml:"CodiceFiscale,omitempty" json:"CodiceFiscale" validate:"omitempty,min=11,max=16,isFiscalCode"`
	Anagrafica    *Anagrafica   `xml:"Anagrafica,omitempty" json:"Anagrafica"`
}

type FatturaElettronicaHeader struct {
	DatiTrasmissione                     *DatiTrasmissione                     `xml:"DatiTrasmissione,omitempty" json:"DatiTrasmissione"`
	CedentePrestatore                    *CedentePrestatore                    `xml:"CedentePrestatore,omitempty" json:"CedentePrestatore"`
	RappresentanteFiscale                *RappresentanteFiscale                `xml:"RappresentanteFiscale,omitempty" json:"RappresentanteFiscale" validate:"omitempty"`
	CessionarioCommittente               *CessionarioCommittente               `xml:"CessionarioCommittente,omitempty" json:"CessionarioCommittente"`
	TerzoIntermediarioOSoggettoEmittente *TerzoIntermediarioOSoggettoEmittente `xml:"TerzoIntermediarioOSoggettoEmittente,omitempty" json:"TerzoIntermediarioOSoggettoEmittente" validate:"omitempty"`
	SoggettoEmittente                    SoggettoEmittente                     `xml:"SoggettoEmittente,omitempty" json:"SoggettoEmittente" validate:"omitempty,len=2,oneof=CC CZ"`
}

type Allegati struct {
	NomeAttachment        string `xml:"NomeAttachment,omitempty" json:"NomeAttachment" validate:"max=60"`
	AlgoritmoCompressione string `xml:"AlgoritmoCompressione,omitempty" json:"AlgoritmoCompressione" validate:"max=10"`
	FormatoAttachment     string `xml:"FormatoAttachment,omitempty" json:"FormatoAttachment" validate:"max=10"`
	DescrizioneAttachment string `xml:"DescrizioneAttachment,omitempty" json:"DescrizioneAttachment" validate:"max=100"`
	Attachment            string `xml:"Attachment,omitempty" json:"Attachment" validate:"base64"`
}

type AltriDatiGestionali struct {
	TipoDato          string `xml:"TipoDato,omitempty" json:"TipoDato" validate:"max=10"`
	RiferimentoTesto  string `xml:"RiferimentoTesto,omitempty" json:"RiferimentoTesto" validate:"max=60"`
	RiferimentoNumero string `xml:"RiferimentoNumero,omitempty" json:"RiferimentoNumero" `
	RiferimentoData   string `xml:"RiferimentoData,omitempty" json:"RiferimentoData" validate:"isDate"`
}

type CodiceArticolo struct {
	CodiceTipo   string `xml:"CodiceTipo,omitempty" json:"CodiceTipo" validate:"max=35"`
	CodiceValore string `xml:"CodiceValore,omitempty" json:"CodiceValore" validate:"max=35"`
}

type DatiAnagraficiVettore struct {
	Anagrafica   *Anagrafica   `xml:"Anagrafica,omitempty" json:"Anagrafica"`
	IDFiscaleIVA *IdFiscaleIVA `xml:"IdFiscaleIVA,omitempty" json:"IdFiscaleIVA"`
}

type DatiRiepilogo struct {
	//DatiIVA        *DatiIVA `xml:"DatiIVA,omitempty" json:"DatiIVA"`
	AliquotaIVA       F64    `xml:"AliquotaIVA" json:"AliquotaIVA" validiate:"isIva"`
	Natura            Natura `xml:"Natura,omitempty" json:"Natura" validate:"omitempty,isNatura"`
	SpeseAccessorie   F64    `xml:"SpeseAccessorie,omitempty" json:"SpeseAccessorie"`
	Arrotondamento    F64    `xml:"Arrotondamento,omitempty" json:"Arrotondamento"`
	ImponibileImporto F64    `xml:"ImponibileImporto" json:"ImponibileImporto"`
	Imposta           F64    `xml:"Imposta" json:"Imposta"`

	EsigibilitaIVA       EsigibilitaIVA `xml:"EsigibilitaIVA,omitempty" json:"EsigibilitaIVA" validate:"omitempty,oneof=I D S"`
	RiferimentoNormativo string         `xml:"RiferimentoNormativo,omitempty" json:"RiferimentoNormativo"`
}

type DettaglioLinee struct {
	NumeroLinea                int                    `xml:"NumeroLinea,omitempty" json:"NumeroLinea" validate:"min=1,max=9999"`
	TipoCessionePrestazione    string                 `xml:"TipoCessionePrestazione,omitempty" json:"TipoCessionePrestazione" validate:"isTypeCassaP"`
	CodiceArticolo             *CodiceArticolo        `xml:"CodiceArticolo,omitempty" json:"CodiceArticolo"`
	Descrizione                string                 `xml:"Descrizione,omitempty" json:"Descrizione" validate:"required,max=1000"`
	Quantita                   F64                    `xml:"Quantita,omitempty" json:"Quantita" validate:"max=21,min=4"`
	UnitaMisura                string                 `xml:"UnitaMisura,omitempty" json:"UnitaMisura" validate:"max=10"`
	DataInizioPeriodo          Date                   `xml:"DataInizioPeriodo,omitempty" json:"DataInizioPeriodo" validate:"isDate"`
	DataFinePeriodo            Date                   `xml:"DataFinePeriodo,omitempty" json:"DataFinePeriodo" validate:"isDate"`
	PrezzoUnitario             F64                    `xml:"PrezzoUnitario,omitempty" json:"PrezzoUnitario" validate:"min=4,max=21"`
	ScontoMaggiorazione        *ScontoMaggiorazione   `xml:"ScontoMaggiorazione,omitempty" json:"ScontoMaggiorazione"`
	PrezzoTotale               F64                    `xml:"PrezzoTotale,omitempty" json:"PrezzoTotale" validate:"min=4,max=21"`
	AliquotaIVA                F64                    `xml:"AliquotaIVA" json:"AliquotaIVA" validate:"isIva"`
	Ritenuta                   string                 `xml:"Ritenuta,omitempty" json:"Ritenuta" validate:"omitempty,eq=SI"`
	Natura                     Natura                 `xml:"Natura,omitempty" json:"Natura" validate:"isNatura"`
	RiferimentoAmministrazione string                 `xml:"RiferimentoAmministrazione,omitempty" json:"RiferimentoAmministrazione" validate:"max=20"`
	AltriDatiGestionali        []*AltriDatiGestionali `xml:"AltriDatiGestionali,omitempty" json:"AltriDatiGestionali"`
}

type DatiBeniServizi struct {
	DettaglioLinee []*DettaglioLinee `xml:"DettaglioLinee,omitempty" json:"DettaglioLinee" `
	DatiRiepilogo  []*DatiRiepilogo  `xml:"DatiRiepilogo,omitempty" json:"DatiRiepilogo"`
}

type DatiBollo struct {
	BolloVirtuale string `xml:"BolloVirtuale,omitempty" json:"BolloVirtuale" validate:"eq=SI"`
	ImportoBollo  string `xml:"ImportoBollo,omitempty" json:"ImportoBollo" validate:"max=15"`
}

type DatiCassaPrevidenziale struct {
	TipoCassa                  TipoCassa `xml:"TipoCassa,omitempty" json:"TipoCassa" validate:"isTypeCassa"`
	AlCassa                    string    `xml:"AlCassa,omitempty" json:"AlCassa"`
	ImportoContributoCassa     string    `xml:"ImportoContributoCassa,omitempty" json:"ImportoContributoCassa" validate:"max=15"`
	ImponibileCassa            string    `xml:"ImponibileCassa,omitempty" json:"ImponibileCassa" validate:"max=15"`
	AliquotaIVA                F64       `xml:"AliquotaIVA,omitempty" json:"AliquotaIVA" validate:"isIva"`
	Ritenuta                   string    `xml:"Ritenuta,omitempty" json:"Ritenuta" validate:"eq=SI,len=2"`
	Natura                     Natura    `xml:"Natura,omitempty" json:"Natura" validate:"isNatura"`
	RiferimentoAmministrazione string    `xml:"RiferimentoAmministrazione,omitempty" json:"RiferimentoAmministrazione" validate:"max=20"`
}

type DatiDocumentiCorrelatiType struct {
	RiferimentoNumeroLinea    int    `xml:"RiferimentoNumeroLinea,omitempty" json:"RiferimentoNumeroLinea" validate:"min=0,max=9999"`
	IDDocumento               string `xml:"IdDocumento,omitempty" json:"IdDocumento" validate:"max=20"`
	Data                      Date   `xml:"Data,omitempty" json:"Data" validate:"omitempty,isDate"`
	NumItem                   string `xml:"NumItem,omitempty" json:"NumItem" validate:""`
	CodiceCommessaConvenzione string `xml:"CodiceCommessaConvenzione,omitempty" json:"CodiceCommessaConvenzione" validate:"max=100"`
	CodiceCUP                 string `xml:"CodiceCUP,omitempty" json:"CodiceCUP" validate:"max=15"`
	CodiceCIG                 string `xml:"CodiceCIG,omitempty" json:"CodiceCIG" validate:"max=15"`
}
type DatiConvenzione = DatiDocumentiCorrelatiType
type DatiOrdineAcquisto = DatiDocumentiCorrelatiType
type DatiContratto = DatiDocumentiCorrelatiType
type DatiFattureCollegate = DatiDocumentiCorrelatiType
type DatiRicezione = DatiDocumentiCorrelatiType

type DatiDDT struct {
	NumeroDDT              string `xml:"NumeroDDT,omitempty" json:"NumeroDDT" validate:"max=20"`
	DataDDT                string `xml:"DataDDT,omitempty" json:"DataDTT" validate:"isDate"`
	RiferimentoNumeroLinea int    `xml:"RiferimentoNumeroLinea,omitempty" json:"RiferimentoNumeroLinea" validate:"min=0,max=9999"`
}

type DatiGeneraliDocumento struct {
	TipoDocumento          TipoDocumento           `xml:"TipoDocumento,omitempty" json:"TipoDocumento" validate:"isTypeDocument"`
	Divisa                 string                  `xml:"Divisa,omitempty" json:"Divisa" validate:"len=3"`
	Data                   Date                    `xml:"Data,omitempty" json:"Data" validate:"isDate,noFuture"`
	Numero                 string                  `xml:"Numero,omitempty" json:"Numero"`
	DatiRitenuta           []*DatiRitenuta         `xml:"DatiRitenuta,omitempty" json:"DatiRitenuta" validate:"omitempty"`
	DatiBollo              *DatiBollo              `xml:"DatiBollo,omitempty" json:"DatiBollo" validate:"omitempty"`
	DatiCassaPrevidenziale *DatiCassaPrevidenziale `xml:"DatiCassaPrevidenziale,omitempty" json:"DatiCassaPrevidenziale" `
	ScontoMaggiorazione    *ScontoMaggiorazione    `xml:"ScontoMaggiorazione,omitempty" json:"ScontoMaggiorazione" `
	ImportoTotaleDocumento F64                     `xml:"ImportoTotaleDocumento,omitempty" json:"ImportoTotaleDocumento" validate:""`
	Arrotondamento         F64                     `xml:"Arrotondamento,omitempty" json:"Arrotondamento" validate:""`
	Causale                []string                `xml:"Causale,omitempty" json:"Causale"`
	Art73                  string                  `xml:"Art73,omitempty" json:"Art73" validate:"omitempty,eq=SI"`
}

type DatiGenerali struct {
	DatiGeneraliDocumento *DatiGeneraliDocumento `xml:"DatiGeneraliDocumento,omitempty" json:"DatiGeneraliDocumento"`
	DatiOrdineAcquisto    *DatiOrdineAcquisto    `xml:"DatiOrdineAcquisto,omitempty" json:"DatiOrdineAcquisto,omitempty"`
	DatiContratto         *DatiContratto         `xml:"DatiContratto,omitempty" json:"DatiContratto"`
	DatiConvenzione       *DatiConvenzione       `xml:"DatiConvenzione,omitempty" json:"DatiConvenzione"`
	DatiRicezione         *DatiRicezione         `xml:"DatiRicezione,omitempty" json:"DatiRicezione"`
	DatiFattureCollegate  *DatiFattureCollegate  `xml:"DatiFattureCollegate,omitempty" json:"DatiFattureCollegate"`
	DatiSAL               *DatiSAL               `xml:"DatiSAL,omitempty" json:"DatiSAL"`
	DatiDDT               *DatiDDT               `xml:"DatiDDT,omitempty" json:"DatiDDT"`
	DatiTrasporto         *DatiTrasporto         `xml:"DatiTrasporto,omitempty" json:"DatiTrasporto" validate:"omitempty"`
	FatturaPrincipale     *FatturaPrincipale     `xml:"FatturaPrincipale,omitempty" json:"FatturaPrincipale"`
}

type DatiIVA struct {
	AliquotaIVA float64 `xml:"Aliquota,omitempty" json:"Aliquota" validate:"isIva"`
	Imposta     float64 `xml:"Imposta,omitempty" json:"Imposta" `
}

type DatiPagamento struct {
	CondizioniPagamento string              `xml:"CondizioniPagamento,omitempty" json:"CondizioniPagamento" validate:"isConditionPayment"`
	DettaglioPagamento  *DettaglioPagamento `xml:"DettaglioPagamento,omitempty" json:"DettaglioPagamento"`
}

type DatiRitenuta struct {
	TipoRitenuta     string `xml:"TipoRitenuta,omitempty" json:"TipoRitenuta" validate:"isTypeRitenuta"`
	ImportoRitenuta  F64    `xml:"ImportoRitenuta,omitempty" json:"ImportoRitenuta" `
	AliquotaRitenuta F64    `xml:"AliquotaRitenuta,omitempty" json:"AliquotaRitenuta"`
	CausalePagamento string `xml:"CausalePagamento,omitempty" json:"CausalePagamento" validate:"isConditionPayment"`
}

type DatiSAL struct {
	RiferimentoFase int `xml:"RiferimentoFase,omitempty" json:"RiferimentoFase" validate:"min=0,max=999"`
}

type DatiTrasporto struct {
	DatiAnagraficiVettore *DatiAnagraficiVettore `xml:"DatiAnagraficiVettore,omitempty" json:"DatiAnagraficiVettore"`
	MezzoTrasporto        string                 `xml:"MezzoTrasporto,omitempty" json:"MezzoTrasporto" validate:"omitempty,max=80"`
	CausaleTrasporto      string                 `xml:"CausaleTrasporto,omitempty" json:"CausaleTrasporto" validate:"omitempty,max=100"`
	NumeroColli           int                    `xml:"NumeroColli,omitempty" json:"NumeroColli" validate:"omitempty,max=9999,min=0"`
	Descrizione           string                 `xml:"Descrizione,omitempty" json:"Descrizione" validate:"omitempty,max=100"`
	UnitaMisuraPeso       string                 `xml:"UnitaMisuraPeso,omitempty" json:"UnitaMisuraPeso" validate:"omitempty,max=10"`
	PesoLordo             string                 `xml:"PesoLordo,omitempty" json:"PesoLordo" validate:"omitempty,min=4,max=7"`
	PesoNetto             string                 `xml:"PesoNetto,omitempty" json:"PesoNetto" validate:"omitempty,min=4,max=7"`
	DataOraRitiro         string                 `xml:"DataOraRitiro,omitempty" json:"DataOraRitiro" validate:"omitempty,isDateTime"`
	DataInizioTrasporto   string                 `xml:"DataInizioTrasporto,omitempty" json:"DataInizioTrasporto" validate:"omitempty,isDate"`
	TipoResa              string                 `xml:"TipoResa,omitempty" json:"TipoResa" validate:"omitempty,len=3,oneof=EXW FCA CPT CIP DAT DAP DDP FAS FOB CFR CIF"` //TODO: da implementare il validate
	IndirizzoResa         *IndirizzoType         `xml:"IndirizzoResa,omitempty" json:"IndirizzoResa" `
	DataOraConsegna       string                 `xml:"DataOraConsegna,omitempty" json:"DataOraConsegna" validate:"isDateTime"`
}

type DatiVeicolo struct {
	Data           string `xml:"Data,omitempty" json:"Data" validate:"isDate"`
	TotalePercorso string `xml:"TotalePercorso,omitempty" json:"Tot alePercorso" validate:"max=15"`
}

type DettaglioPagamento struct {
	Beneficiario                    string          `xml:"Beneficiario,omitempty" json:"Beneficiario" validate:"max=200"`
	ModalitaPagamento               MetodoPagamento `xml:"ModalitaPagamento,omitempty" json:"ModalitaPagamento" validate:"isMP"`
	DataRiferimentoTerminiPagamento Date            `xml:"DataRiferimentoTerminiPagamento,omitempty" json:"DataRiferimentoTerminiPagamento" validate:"omitempty,isDate"`
	GiorniTerminiPagamento          int             `xml:"GiorniTerminiPagamento,omitempty" json:"GiorniTerminiPagamento,omitempty" validate:"omitempty,min=0,max=999"`
	DataScadenzaPagamento           string          `xml:"DataScadenzaPagamento,omitempty" json:"DataScadenzaPagamento" validate:"omitempty,isDate"`
	ImportoPagamento                float64         `xml:"ImportoPagamento,omitempty" json:"ImportoPagamento" validate:"min=4,max=15"`
	CodUfficioPostale               string          `xml:"CodUfficioPostale,omitempty" json:"CodUfficioPostale" validate:"omitempty,max=20"`
	CognomeQuietanzante             string          `xml:"CognomeQuietanzante,omitempty" json:"CognomeQuietanzante" validate:"omitempty,max=60"`
	NomeQuietanzante                string          `xml:"NomeQuietanzante,omitempty" json:"NomeQuietanzante" validate:"omitempty,max=60"`
	CFQuietanzante                  string          `xml:"CFQuietanzante,omitempty" json:"CFQuietanzante" validate:"omitempty,max=16"`
	TitoloQuietanzante              string          `xml:"TitoloQuietanzante,omitempty" json:"TitoloQuietanzante" validate:"omitempty,max=10,min=2"`
	IstitutoFinanziario             string          `xml:"IstitutoFinanziario,omitempty" json:"IstitutoFinanziario" validate:"omitempty,max=80"`

	IBAN string `xml:"IBAN,omitempty" json:"IBAN" validate:"omitempty,min=15,max=34,required_with_all=ABI CAB BIC"`
	ABI  string `xml:"ABI,omitempty" json:"ABI" validate:"omitempty,len=5,required_with_all=IBAN CAB BIC"`
	CAB  string `xml:"CAB,omitempty" json:"CAB" validate:"omitempty,len=5,required_with_all=ABI IBAN BIC"`
	BIC  string `xml:"BIC,omitempty" json:"BIC" validate:"omitempty,min=8,max=11,required_with_all=ABI CAB IBAN"`

	ScontoPagamentoAnticipato     F64    `xml:"ScontoPagamentoAnticipato,omitempty" json:"ScontoPagamentoAnticipato" validate:"omitempty"`
	DataLimitePagamentoAnticipato string `xml:"DataLimitePagamentoAnticipato,omitempty" json:"DataLimitePagamentoAnticipato" validate:"omitempty,isDate"`
	PenalitaPagamentiRitardati    F64    `xml:"PenalitaPagamentiRitardati,omitempty" json:"PenalitaPagamentiRitardati" validate:"omitempty"`
	DataDecorrenzaPenale          string `xml:"DataDecorrenzaPenale,omitempty" json:"DataDecorrenzaPenale" validate:"omitempty,isDate"`
	CodicePagamento               string `xml:"CodicePagamento,omitempty" json:"CodicePagamento" validate:"max=60"`
}

type FatturaPrincipale struct {
	NumeroFatturaPrincipale string `xml:"NumeroFatturaPrincipale,omitempty" json:"NumeroFatturaPrincipale" validate:"max=20"`
	DataFatturaPrincipale   string `xml:"DataFatturaPrincipale,omitempty" json:"DataFatturaPrincipale" validate:"isDate"`
}

type ScontoMaggiorazione struct {
	Tipo        ScontoMaggiorazioneType `xml:"Tipo,omitempty" json:"Tipo" validate:"oneof=SC MG"`
	Percentuale float64                 `xml:"Percentuale,omitempty" json:"Percentuale"`
	Importo     float64                 `xml:"Importo,omitempty" json:"Importo"`
}

// FatturaElettronicaBody represents the body of an electronic invoice in all its parts
type FatturaElettronicaBody struct {
	DatiGenerali    *DatiGenerali    `xml:"DatiGenerali,omitempty" json:"DatiGenerali"`
	DatiBeniServizi *DatiBeniServizi `xml:"DatiBeniServizi,omitempty" json:"DatiBeniServizi"`
	DatiVeicolo     *DatiVeicolo     `xml:"DatiVeicolo,omitempty" json:"DatiVeicolo"`
	DatiPagamento   *DatiPagamento   `xml:"DatiPagamento,omitempty" json:"DatiPagamento"`
	Allegati        []*Allegati      `xml:"Allegati,omitempty" json:"Allegati" validate:"dive"`
}

// FatturaElettronica The structure of a multi body electronic invoice, if you need only one body,
// just insert a single element in the FatturaElettronicaBod slice...
type FatturaElettronica struct {
	XMLName                  xml.Name
	Versione                 FormatoTrasmissione       `xml:"versione,attr" json:"Versione"`
	FatturaElettronicaHeader *FatturaElettronicaHeader `xml:"FatturaElettronicaHeader,omitempty" json:"FatturaElettronicaHeader"`
	FatturaElettronicaBody   []*FatturaElettronicaBody `xml:"FatturaElettronicaBody,omitempty" json:"FatturaElettronicaBody"`
	Signature                string                    `xml:"ds:Signature,omitempty" json:"Signature"`
	Xmlns                    xml.Attr                  `xml:",attr" json:"Xmlns"`
}
