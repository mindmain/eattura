package error_sdi

import (
	"fmt"

	"github.com/mindmain/eattura/sdi/fe"
	"github.com/pkg/errors"
)

type Code string
type SDICheck func(invoice *fe.FatturaElettronica) bool

var (
	errorSdi = errors.New("sdi error")
)

var allErrors = make(map[Code]SdiError)

type SdiError interface {
	Error() string
	Code() Code
	Reason() string
	Has(invoice *fe.FatturaElettronica) bool
}

type sdiError struct {
	reason  string
	code    Code
	checker SDICheck
}

func New(code Code, checker SDICheck, reason string) SdiError {

	err := &sdiError{
		reason:  reason,
		code:    code,
		checker: checker,
	}

	if _, ok := allErrors[code]; !ok {
		allErrors[code] = err
	} else {
		panic(fmt.Sprintf("error code %s already exists", code))
	}

	return err
}

func (e *sdiError) Error() string {
	return errors.Wrap(errorSdi, fmt.Sprintf("[%s] %s", e.code, e.reason)).Error()
}

func (e *sdiError) Code() Code {
	return e.code
}

func (e *sdiError) Reason() string {
	return e.reason
}

func (e *sdiError) Has(invoice *fe.FatturaElettronica) bool {
	if e.checker != nil {
		return e.checker(invoice)
	}
	// if the checker is nil, omit this error
	return false
}

// CheckInvoice check if the invoice is valid and return a list of errors if any is found, and a boolean that is true if the invoice is valid
func CheckInvoice(invoice *fe.FatturaElettronica) ([]SdiError, bool) {

	var errors []SdiError

	for _, err := range allErrors {
		if err.Has(invoice) {
			errors = append(errors, err)
		}
	}

	return errors, len(errors) == 0
}
