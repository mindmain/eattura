package sdi

import (
	"io"
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

func testInvoiceOnlyFilename() io.Reader {
	return strings.NewReader(`<?xml version="1.0" encoding="UTF-8"?>
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
			<CedentePrestatore>
				<DatiAnagrafici>
					<IdFiscaleIVA>
						<IdPaese>IT</IdPaese>
						<IdCodice>12345678901</IdCodice>
					</IdFiscaleIVA>
					<Anagrafica>
						<Denominazione>Denominazione</Denominazione>
					</Anagrafica>
				</DatiAnagrafici>
			</CedentePrestatore>
		</FatturaElettronicaHeader>
	</FatturaElettronica>`)
}

func TestReadFromFile(t *testing.T) {

	reader := testInvoiceOnlyFilename()

	fe, err := Read("IT12345678901_12345.xml", reader)

	if err != nil {
		t.Fatal(err)
	}

	assert.NoError(t, err)
	assert.NotNil(t, fe)
	assert.Equal(t, "IT", fe.FatturaElettronicaHeader.DatiTrasmissione.IdTrasmittente.IdPaese)
	assert.Equal(t, "01234567890", fe.FatturaElettronicaHeader.DatiTrasmissione.IdTrasmittente.IdCodice)

	filename, err := fe.Filename()

	if err != nil {
		t.Fatal(err)
	}

	assert.Equal(t, "IT12345678901_12345.xml", filename)
}

func TestFilename(t *testing.T) {

	t.Run("test wrong filename", func(t *testing.T) {
		_, err := Read("IT12345678901_123456.xml", nil)
		assert.Error(t, err)
		assert.Equal(t, ErrInvalidFileName, err)
	})

	t.Run("test wrong filename", func(t *testing.T) {
		_, err := Read("IT12345678901.xml", nil)
		assert.Error(t, err)
		assert.Equal(t, ErrInvalidFileName, err)
	})

}
