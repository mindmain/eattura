package sdi

import (
	"encoding/xml"
	"errors"
	"io"
	"strings"

	"github.com/mindmain/eattura/sdi/fe"
)

var ErrInvalidUniqueLength = errors.New("invalid unique length must be 5 characters after underscore")
var ErrInvalidFileName = errors.New("invalid file name")
var ErrReaderNil = errors.New("reader is nil")

func Read(filename string, f io.Reader) (*fe.FatturaElettronica, error) {

	if !IsInvoiceFileName(filename) && !IsInvoiceFileNameCrypt(filename) {
		return nil, ErrInvalidFileName
	}

	filename = strings.Replace(filename, ".p7m", "", -1)
	filename = strings.Replace(filename, ".xml", "", -1)

	partNames := strings.Split(filename, "_")

	if len(partNames) != 2 {
		return nil, ErrInvalidFileName
	}

	unique := partNames[1]

	if len(unique) != 5 {
		return nil, ErrInvalidUniqueLength
	}

	if f == nil {
		return nil, ErrReaderNil
	}

	var fe fe.FatturaElettronica

	xmlDecoder := xml.NewDecoder(f)

	err := xmlDecoder.Decode(&fe)

	if err != nil {
		return nil, err
	}
	var uniqueBytes [5]byte

	copy(uniqueBytes[:], []byte(unique)[:5])

	fe.SetUniqueFileName(uniqueBytes)

	return &fe, nil

}
