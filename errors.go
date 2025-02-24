package eattura

import "errors"

var ErrInvoiceNotModifiable = errors.New("invoice not modifiable")
var ErrNilRequest = errors.New("nil request")
var ErrCannotChangeContactVatCode = errors.New("cannot change contact vat code")
var ErrCannotDeleteContactWithInvoice = errors.New("cannot delete contact with invoice")
