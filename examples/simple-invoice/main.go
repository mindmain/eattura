package main

import (
	"fmt"
	"time"

	"github.com/mindmain/eattura"
)

func main() {

	fe := &eattura.Invoice{
		Date: time.Date(2021, 1, 1, 0, 0, 0, 0, time.UTC),

		Issuer: &eattura.Issuer{
			ContactData: eattura.ContactData{
				Name:    "Mario",
				Surname: "Rossi",
				Email:   "mario@fake.email.com",
				Phone:   "1234567890",
				VatCode: &eattura.VatCode{
					Code:   "12345678901",
					Nation: "IT",
				},

				Address: &eattura.Address{
					Country:  "IT",
					Province: "RM",
					City:     "Roma",
					Street:   "Via Roma",
					Number:   "1",
					ZipCode:  "00100",
				},
				FiscalCode: "RSSMRA80A01H501A",
			},
		},
		Supplier: &eattura.Supplier{
			ContactData: eattura.ContactData{
				Name:    "Mario",
				Surname: "Rossi",
				Phone:   "1234567890",
				VatCode: &eattura.VatCode{
					Code:   "12345678901",
					Nation: "IT",
				},

				Address: &eattura.Address{
					Country:  "IT",
					Province: "RM",
					City:     "Roma",
					Street:   "Via Roma",
					Number:   "1",
					ZipCode:  "00100",
				},

				FiscalCode: "RSSMRA80A01H501A",
			},
		},

		Customer: &eattura.Customer{
			ContactData: eattura.ContactData{
				Name:    "Mario",
				Surname: "Rossi",
				Phone:   "1234567890",
				VatCode: &eattura.VatCode{
					Code:   "12345678901",
					Nation: "IT",
				},

				Address: &eattura.Address{
					Country:  "IT",
					Province: "RM",

					City:    "Roma",
					Street:  "Via Roma",
					Number:  "1",
					ZipCode: "00100",
				},

				FiscalCode: "RSSMRA80A01H501A",
			},
		},

		Items: []*eattura.InvoiceItem{
			{
				Description: "Test",
				Qty:         1,
				UnitAmount:  100,
				Unit:        "pz",
				VatAmount:   22,
			},
			{
				Description: "Test with zero vat",
				Qty:         1,
				UnitAmount:  100,
				Unit:        "pz",
				VatAmount:   0,
				Nature:      eattura.NatureNoIvaOtherCases,
			},
		},
	}

	bb, err := fe.MarshalIndent("", "  ")

	if err != nil {
		panic(err)
	}

	fmt.Printf("%s\n", bb)

	/* Output:
		<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
	<ns2:FatturaElettronica versione="FPR12" xmlns:ns2="http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fatture/v1.2">
	  <FatturaElettronicaHeader>
	    <DatiTrasmissione>
	      <IdTrasmittente>
	        <IdPaese>IT</IdPaese>
	        <IdCodice>12345678901</IdCodice>
	      </IdTrasmittente>
	      <FormatoTrasmissione>FPR12</FormatoTrasmissione>
	      <CodiceDestinatario>0000000</CodiceDestinatario>
	    </DatiTrasmissione>
	    <CedentePrestatore>
	      <DatiAnagrafici>
	        <IdFiscaleIVA>
	          <IdPaese>IT</IdPaese>
	          <IdCodice>12345678901</IdCodice>
	        </IdFiscaleIVA>
	        <Anagrafica>
	          <Nome>Mario</Nome>
	          <Cognome>Rossi</Cognome>
	        </Anagrafica>
	      </DatiAnagrafici>
	      <Sede>
	        <Indirizzo>Via Roma</Indirizzo>
	        <NumeroCivico>1</NumeroCivico>
	        <CAP>00100</CAP>
	        <Comune>Roma</Comune>
	        <Provincia>RM</Provincia>
	        <Nazione>IT</Nazione>
	      </Sede>
	      <Contatti>
	        <Telefono>1234567890</Telefono>
	      </Contatti>
	    </CedentePrestatore>
	    <CessionarioCommittente>
	      <DatiAnagrafici>
	        <IdFiscaleIVA>
	          <IdPaese>IT</IdPaese>
	          <IdCodice>12345678901</IdCodice>
	        </IdFiscaleIVA>
	        <Anagrafica>
	          <Nome>Mario</Nome>
	          <Cognome>Rossi</Cognome>
	        </Anagrafica>
	      </DatiAnagrafici>
	      <Sede>
	        <Indirizzo>Via Roma</Indirizzo>
	        <NumeroCivico>1</NumeroCivico>
	        <CAP>00100</CAP>
	        <Comune>Roma</Comune>
	        <Provincia>RM</Provincia>
	        <Nazione>IT</Nazione>
	      </Sede>
	    </CessionarioCommittente>
	  </FatturaElettronicaHeader>
	  <FatturaElettronicaBody>
	    <DatiGenerali>
	      <DatiGeneraliDocumento>
	        <Divisa>EUR</Divisa>
	        <Data>2021-01-01</Data>
	      </DatiGeneraliDocumento>
	    </DatiGenerali>
	    <DatiBeniServizi>
	      <DettaglioLinee>
	        <Descrizione>Test</Descrizione>
	        <Quantita>1.00</Quantita>
	        <UnitaMisura>pz</UnitaMisura>
	        <PrezzoUnitario>100.00</PrezzoUnitario>
	        <PrezzoTotale>100.00</PrezzoTotale>
	        <AliquotaIVA>22.00</AliquotaIVA>
	      </DettaglioLinee>
	      <DettaglioLinee>
	        <NumeroLinea>1</NumeroLinea>
	        <Descrizione>Test with zero vat</Descrizione>
	        <Quantita>1.00</Quantita>
	        <UnitaMisura>pz</UnitaMisura>
	        <PrezzoUnitario>100.00</PrezzoUnitario>
	        <PrezzoTotale>100.00</PrezzoTotale>
	        <AliquotaIVA>0.00</AliquotaIVA>
	        <Natura>N2.2</Natura>
	      </DettaglioLinee>
	      <DatiRiepilogo>
	        <AliquotaIVA>22.00</AliquotaIVA>
	        <ImponibileImporto>100.00</ImponibileImporto>
	        <Imposta>22.00</Imposta>
	      </DatiRiepilogo>
	    </DatiBeniServizi>
	  </FatturaElettronicaBody><?xml version="1.0" encoding="UTF-8" standalone="yes"?>
	</ns2:FatturaElettronica>
	➜  eattura (main) go run ./examples/simple-invoice                                                                                                                                                                                                                                                                            ✭ ✱
	<?xml version="1.0" encoding="UTF-8"?>
	  <ns2:FatturaElettronica versione="FPR12" xmlns:ns2="http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fatture/v1.2">
	    <FatturaElettronicaHeader>
	      <DatiTrasmissione>
	        <IdTrasmittente>
	          <IdPaese>IT</IdPaese>
	          <IdCodice>12345678901</IdCodice>
	        </IdTrasmittente>
	        <FormatoTrasmissione>FPR12</FormatoTrasmissione>
	        <CodiceDestinatario>0000000</CodiceDestinatario>
	      </DatiTrasmissione>
	      <CedentePrestatore>
	        <DatiAnagrafici>
	          <IdFiscaleIVA>
	            <IdPaese>IT</IdPaese>
	            <IdCodice>12345678901</IdCodice>
	          </IdFiscaleIVA>
	          <Anagrafica>
	            <Nome>Mario</Nome>
	            <Cognome>Rossi</Cognome>
	          </Anagrafica>
	        </DatiAnagrafici>
	        <Sede>
	          <Indirizzo>Via Roma</Indirizzo>
	          <NumeroCivico>1</NumeroCivico>
	          <CAP>00100</CAP>
	          <Comune>Roma</Comune>
	          <Provincia>RM</Provincia>
	          <Nazione>IT</Nazione>
	        </Sede>
	        <Contatti>
	          <Telefono>1234567890</Telefono>
	        </Contatti>
	      </CedentePrestatore>
	      <CessionarioCommittente>
	        <DatiAnagrafici>
	          <IdFiscaleIVA>
	            <IdPaese>IT</IdPaese>
	            <IdCodice>12345678901</IdCodice>
	          </IdFiscaleIVA>
	          <Anagrafica>
	            <Nome>Mario</Nome>
	            <Cognome>Rossi</Cognome>
	          </Anagrafica>
	        </DatiAnagrafici>
	        <Sede>
	          <Indirizzo>Via Roma</Indirizzo>
	          <NumeroCivico>1</NumeroCivico>
	          <CAP>00100</CAP>
	          <Comune>Roma</Comune>
	          <Provincia>RM</Provincia>
	          <Nazione>IT</Nazione>
	        </Sede>
	      </CessionarioCommittente>
	    </FatturaElettronicaHeader>
	    <FatturaElettronicaBody>
	      <DatiGenerali>
	        <DatiGeneraliDocumento>
	          <Divisa>EUR</Divisa>
	          <Data>2021-01-01</Data>
	        </DatiGeneraliDocumento>
	      </DatiGenerali>
	      <DatiBeniServizi>
	        <DettaglioLinee>
	          <Descrizione>Test</Descrizione>
	          <Quantita>1.00</Quantita>
	          <UnitaMisura>pz</UnitaMisura>
	          <PrezzoUnitario>100.00</PrezzoUnitario>
	          <PrezzoTotale>100.00</PrezzoTotale>
	          <AliquotaIVA>22.00</AliquotaIVA>
	        </DettaglioLinee>
	        <DettaglioLinee>
	          <NumeroLinea>1</NumeroLinea>
	          <Descrizione>Test with zero vat</Descrizione>
	          <Quantita>1.00</Quantita>
	          <UnitaMisura>pz</UnitaMisura>
	          <PrezzoUnitario>100.00</PrezzoUnitario>
	          <PrezzoTotale>100.00</PrezzoTotale>
	          <AliquotaIVA>0.00</AliquotaIVA>
	          <Natura>N2.2</Natura>
	        </DettaglioLinee>
	        <DatiRiepilogo>
	          <AliquotaIVA>22.00</AliquotaIVA>
	          <ImponibileImporto>100.00</ImponibileImporto>
	          <Imposta>22.00</Imposta>
	        </DatiRiepilogo>
	      </DatiBeniServizi>
	    </FatturaElettronicaBody>
	  </ns2:FatturaElettronica>
	*/
}
