package sdi

import (
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestIsInvoiceFilename(t *testing.T) {

	assert.False(t, IsInvoiceFileName("IT0123_12345.xml"))
	assert.True(t, IsInvoiceFileName("IT00000000000_12345.xml"))
	assert.False(t, IsInvoiceFileName("IT012345678901234567890123456789_12345.xml"))
	assert.False(t, IsInvoiceFileName("IT012345678901234567890123456789_12345.xml"))

}

func TestReadFromFile(t *testing.T) {

	reader := strings.NewReader(`<?xml version="1.0" encoding="UTF-8"?>
	<FatturaElettronica xmlns="http://ivaservizi.agenziaentrate.gov.it/docs/xsd/fatture/v1.2" versione="FPR12">
		<FatturaElettronicaHeader>
			<DatiTrasmissione>
				<IdTrasmittente>
					<IdPaese>IT</IdPaese>
					<IdCodice>01234567890</IdCodice>
				</IdTrasmittente>
				<ProgressivoInvio>00001</ProgressivoInvio>
				<FormatoTrasmissione>FPR12</FormatoTrasmissione>
			</DatiTrasmissione>
		</FatturaElettronicaHeader>
	</FatturaElettronica>`)

	fe, err := ReadFromFile(reader)

	assert.Nil(t, err)
	assert.NotNil(t, fe)
	assert.Equal(t, "IT", fe.FatturaElettronicaHeader.DatiTrasmissione.IdTrasmittente.IdPaese)
	assert.Equal(t, "01234567890", fe.FatturaElettronicaHeader.DatiTrasmissione.IdTrasmittente.IdCodice)

}
