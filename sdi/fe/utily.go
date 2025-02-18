package fe

import (
	"encoding/xml"
	"time"
)

func (f *FatturaElettronica) Date() time.Time {

	if len(f.FatturaElettronicaBody) == 0 {
		return time.Time{}
	}

	if f.FatturaElettronicaBody[0].DatiGenerali == nil {
		return time.Time{}
	}

	if f.FatturaElettronicaBody[0].DatiGenerali.DatiGeneraliDocumento == nil {
		return time.Time{}
	}

	return f.FatturaElettronicaBody[0].DatiGenerali.DatiGeneraliDocumento.Data.Time

}

func (f *FatturaElettronica) XmlByte() ([]byte, error) {
	return xml.Marshal(f)
}

func (riepilogo *DatiRiepilogo) Is(n Natura) bool {
	return riepilogo.Natura == n
}

func (dataBeni *DatiBeniServizi) HasNatura(n Natura) (bool, *DatiRiepilogo) {
	for _, riepilogo := range dataBeni.DatiRiepilogo {
		if riepilogo.Is(n) {
			return true, riepilogo
		}
	}
	return false, nil
}

func (datiBeni *DatiBeniServizi) HasAliquota(iva F64) (bool, *DatiRiepilogo) {
	for _, riepilogo := range datiBeni.DatiRiepilogo {
		if riepilogo.AliquotaIVA == iva {
			return true, riepilogo
		}
	}
	return false, nil
}

func (dataBeni *DatiBeniServizi) AddRiepilogo(details *DettaglioLinee, es EsigibilitaIVA) {

	if details.Natura != "" {
		if found, riepilogo := dataBeni.HasNatura(details.Natura); found {
			riepilogo.Imposta += details.PrezzoTotale * (details.AliquotaIVA / 100)
			riepilogo.ImponibileImporto += details.PrezzoTotale

			riepilogo.Natura = details.Natura
			riepilogo.AliquotaIVA = details.AliquotaIVA
		}
	} else {

		if found, riepilogo := dataBeni.HasAliquota(details.AliquotaIVA); found {
			riepilogo.Imposta += details.PrezzoTotale * (details.AliquotaIVA / 100)
			riepilogo.ImponibileImporto += details.PrezzoTotale
		} else {
			dataBeni.DatiRiepilogo = append(dataBeni.DatiRiepilogo, &DatiRiepilogo{
				AliquotaIVA:       details.AliquotaIVA,
				ImponibileImporto: details.PrezzoTotale,
				Imposta:           details.PrezzoTotale * (details.AliquotaIVA / 100),
				EsigibilitaIVA:    es,
			})
		}
	}

}
