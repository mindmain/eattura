package eattura

import (
	"cmp"
	"fmt"

	"github.com/mindmain/eattura/sdi/fe"
)

func (adr *Address) toSdi() *fe.IndirizzoType {

	if adr == nil {
		return &fe.IndirizzoType{}
	}

	return &fe.IndirizzoType{
		Nazione:      adr.Country,
		Provincia:    fe.Province(adr.Province),
		Comune:       adr.City,
		CAP:          adr.ZipCode,
		Indirizzo:    adr.Street,
		NumeroCivico: adr.Number,
	}
}

func (vc *VatCode) toSdi() *fe.IdFiscaleIVA {
	if vc == nil {
		return &fe.IdFiscaleIVA{}
	}

	return &fe.IdFiscaleIVA{
		IdPaese:  vc.Nation,
		IdCodice: vc.Code,
	}
}

func (c *Contact) contactToSdi() *fe.DatiAnagrafici {

	return &fe.DatiAnagrafici{
		IdFiscaleIVA: c.VatCode.toSdi(),
		Anagrafica: &fe.Anagrafica{
			Denominazione: c.Denomination,
			Nome:          c.Name,
			Cognome:       c.Surname,
			Titolo:        c.Title,
			CodEORI:       c.CodEORI,
		},
	}

}

func (inv *Invoice) rea() *fe.IscrizioneREA {

	if inv.Supplier == nil {
		return nil
	}

	if inv.Supplier.Rea != nil {

		var reaPart = inv.Supplier.Rea
		var shareholder fe.SocioUnico = fe.SU

		if !reaPart.IsSoleShareholder {
			shareholder = fe.SM
		}

		var liquidationState fe.StatoLiquidazione = fe.LS

		if !reaPart.InLiquidation {
			liquidationState = fe.LN
		}

		return &fe.IscrizioneREA{
			Ufficio:           inv.Supplier.Rea.Office,
			NumeroREA:         inv.Supplier.Rea.Number,
			CapitaleSociale:   fe.F64(inv.Supplier.Rea.Capital),
			SocioUnico:        shareholder,
			StatoLiquidazione: liquidationState,
		}
	}

	return nil

}

func (inv *Invoice) getBodyInvoice() *fe.FatturaElettronicaBody {

	var dataItems = &fe.DatiBeniServizi{
		DettaglioLinee: []*fe.DettaglioLinee{},
		DatiRiepilogo:  []*fe.DatiRiepilogo{},
	}

	for index, item := range inv.Items {

		var details = &fe.DettaglioLinee{
			NumeroLinea:    index,
			Descrizione:    item.Description,
			Quantita:       fe.F64(item.Qty),
			UnitaMisura:    item.Unit,
			Natura:         item.Nature,
			PrezzoUnitario: fe.F64(item.UnitAmount),
			PrezzoTotale:   fe.F64(item.UnitAmount * item.Qty),
			AliquotaIVA:    fe.F64(22),
		}

		if item.Nature != "" {
			details.AliquotaIVA = 0
		}

		if !item.Start.IsZero() && !item.End.IsZero() {
			details.DataFinePeriodo = fe.Date{
				Time: item.End,
			}

			details.DataInizioPeriodo = fe.Date{
				Time: item.Start,
			}
		}

		dataItems.DettaglioLinee = append(dataItems.DettaglioLinee, details)
		dataItems.AddRiepilogo(details, "")
	}

	var body = &fe.FatturaElettronicaBody{
		DatiGenerali: &fe.DatiGenerali{
			DatiGeneraliDocumento: &fe.DatiGeneraliDocumento{
				Divisa: "EUR",
				Data: fe.Date{
					Time: inv.Date,
				},
				Numero:        inv.Number,
				TipoDocumento: inv.Type,
			},
		},
		DatiBeniServizi: dataItems,
	}

	return body
}

func (inv *Invoice) GetSDI() (*fe.FatturaElettronica, error) {

	if inv.Customer == nil {
		return nil, fmt.Errorf("customer is required ")
	}

	if inv.Supplier == nil {
		return nil, fmt.Errorf("supplier is required")
	}

	if inv.Issuer == nil {
		return nil, fmt.Errorf("issuer is required")
	}

	var fat = &fe.FatturaElettronica{

		FatturaElettronicaHeader: &fe.FatturaElettronicaHeader{
			DatiTrasmissione: &fe.DatiTrasmissione{
				IdTrasmittente: &fe.IdFiscaleIVA{
					IdPaese:  inv.Issuer.VatCode.Nation,
					IdCodice: inv.Issuer.VatCode.Code,
				},
				FormatoTrasmissione: fe.FPR12,
				CodiceDestinatario:  cmp.Or(inv.DestinationCode, "0000000"),
				PECDestinatario:     inv.Customer.Pec,
			},

			CedentePrestatore: &fe.CedentePrestatore{
				Contatti: &fe.Contatti{
					Email:    inv.Supplier.Email,
					Telefono: inv.Supplier.Phone,
				},
				IscrizioneREA:  inv.rea(),
				Sede:           inv.Supplier.Address.toSdi(),
				DatiAnagrafici: inv.Supplier.contactToSdi(),
			},

			CessionarioCommittente: &fe.CessionarioCommittente{
				Sede:                       inv.Customer.Address.toSdi(),
				DatiAnagrafici:             inv.Customer.contactToSdi(),
				StabileOrganizzazione:      nil,
				RappresentanteFiscale:      nil,
				RiferimentoAmministrazione: "",
			},
			TerzoIntermediarioOSoggettoEmittente: nil,
		},

		FatturaElettronicaBody: []*fe.FatturaElettronicaBody{
			inv.getBodyInvoice(),
		},
	}

	return fat, nil
}
