package eattura

import (
	"context"

	"github.com/mindmain/eattura/db"
	"github.com/mindmain/eattura/db/model"
	"github.com/mindmain/eattura/fs"
	"github.com/mindmain/eattura/pec"
	"github.com/mindmain/eattura/sdi/fe"
)

type invoicerHandler struct {
	database db.Database
	pec      pec.Client

	store     fs.InvoiceFileHandler
	generator GeneratorProgressive
	issuer    IssuerHandler
}

func (invHandler *invoicerHandler) CreateInvoice(ctx context.Context, request *RequestCreateInvoice) (*Invoice, error) {

	contact, err := invHandler.database.Contact().Read(ctx, request.CustomerId)

	if err != nil {
		return nil, err
	}

	issuer, err := invHandler.database.Issuer().Read(ctx, request.IssuerId)

	if err != nil {
		return nil, err
	}

	var invoiceModel = &model.Invoice{
		Status:            model.StatusDraft,
		TypeDocument:      string(request.Type),
		IssuerReference:   issuer.UUID,
		Issuer:            issuer,
		CustomerReference: contact.UUID,
		SupplierReference: issuer.ContactReference,
		InvoiceNumber:     request.InvoiceNumber,
		Date:              request.InvoiceDate,
		Items:             make([]*model.InvoiceItem, 0),
	}

	for _, item := range request.Items {
		invoiceModel.Items = append(invoiceModel.Items, &model.InvoiceItem{
			Description: item.Description,
			Nature:      string(item.Nature),
			UnitAmount:  item.UnitAmount,
			Vat:         item.VatAmount,
			Total:       item.Qty * item.UnitAmount,
			Qty:         item.Qty,
			Unit:        item.Unit,
		})
	}

	invoiceModel.UUID = uuidInvoice(invoiceModel)

	err = invHandler.database.Invoice().Create(ctx, invoiceModel)

	if err != nil {
		return nil, err
	}

	return nil, nil

}
func (invHandler *invoicerHandler) ReadInvoice(ctx context.Context, uuid string) (*Invoice, *fe.FatturaElettronica, error) {

	invoice, err := invHandler.database.Invoice().Read(ctx, uuid)

	if err != nil {
		return nil, nil, err
	}

	var result Invoice
	result.fromModel(invoice)

	year := invoice.Date.Year()
	month := invoice.Date.Month()

	file, err := invHandler.store.ReadInvoice(ctx, year, int(month), invoice.File)

	if err != nil {
		return nil, nil, err
	}

	return &result, file, nil

}

func (invHandler *invoicerHandler) UpdateInvoice(ctx context.Context, uuid string, invoice *Invoice) (*Invoice, error) {

	invoiceNow, err := invHandler.database.Invoice().Read(ctx, uuid)

	if err != nil {
		return nil, err
	}

	if invoiceNow.Status != model.StatusDraft && invoiceNow.Status != model.StatusFailed {
		return nil, ErrInvoiceNotModifiable
	}

	up := invoice.toModel()
	err = invHandler.database.Invoice().Update(ctx, uuid, up)

	if err != nil {
		return nil, err
	}

	return nil, nil

}
func (invHandler *invoicerHandler) DeleteInvoice(ctx context.Context, uuid string) error {

	invoice, err := invHandler.database.Invoice().Read(ctx, uuid)

	if err != nil {
		return err
	}

	if invoice.Status != model.StatusDraft && invoice.Status != model.StatusFailed {
		return ErrInvoiceNotModifiable
	}

	err = invHandler.database.Invoice().Delete(ctx, uuid)

	if err != nil {
		return err
	}

	return nil

}
func (invHandler *invoicerHandler) InvoiceFinder() FinderInvoice {
	return nil
}

func (invHandler *invoicerHandler) SendInvoice(ctx context.Context, invoice *Invoice) error {
	return nil
}
