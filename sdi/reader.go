package sdi

import (
	"encoding/xml"
	"io"

	"github.com/mindmain/eattura/sdi/fe"
)

func ReadFromFile(f io.Reader) (*fe.FatturaElettronica, error) {

	var fe fe.FatturaElettronica

	xmlDecoder := xml.NewDecoder(f)

	err := xmlDecoder.Decode(&fe)

	if err != nil {
		return nil, err
	}

	return &fe, nil

}
