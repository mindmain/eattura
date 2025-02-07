package sdi

import (
	"log"
	"regexp"
)

type SDIHandler interface {
	Check() error
}

// IsInvoiceFileName check if the name is a valid invoice file name, with regex
func IsInvoiceFileName(name string) bool {

	match, err := regexp.Match(`^[A-Z]{2}[A-Z-0-9]{11,28}_[0-9A-Za-z]{5}\.xml$`, []byte(name))

	if err != nil {
		log.Printf("[IsInvoiceFileName error]: %s", err)
		return false
	}

	return match

}
