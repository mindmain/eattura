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
