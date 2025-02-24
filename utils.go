package eattura

import (
	"fmt"

	"github.com/google/uuid"
	"github.com/mindmain/eattura/db/model"
)

const NamespaceEattura = "09f350c4-3657-5fdb-99b5-c836345d4488"

func determinateUUID(code *VatCode) string {
	return uuid.NewSHA1(uuid.MustParse(NamespaceEattura), []byte(fmt.Sprintf("%s%s", code.Nation, code.Code))).String()
}

func uuidInvoice(inv *model.Invoice) string {
	return uuid.NewSHA1(uuid.MustParse(NamespaceEattura), []byte(fmt.Sprintf("%d-%s", inv.Date.Year(), inv.InvoiceNumber))).String()
}
