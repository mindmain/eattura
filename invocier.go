package eattura

import (
	"context"
	"time"

	"github.com/mindmain/eattura/internal/db"
	"github.com/mindmain/eattura/internal/db/model"
	"github.com/mindmain/eattura/internal/fs"
	"github.com/mindmain/eattura/internal/pec"
	"github.com/mindmain/eattura/internal/sdi/fe"
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

type invoiceFinder struct {
	database db.Database
	status   []Status
	start    time.Time
	end      time.Time
}

func (invFinder *invoiceFinder) getRequest() *model.RequestSearchInvoice {
	request := &model.RequestSearchInvoice{
		Status: invFinder.status,
	}

	if !invFinder.start.IsZero() {
		request.StartDate = &invFinder.start
	}

	if !invFinder.end.IsZero() {
		request.EndDate = &invFinder.end
	}

	return request
}

func (invFinder *invoiceFinder) WithStatus(status Status) FinderInvoice {
	invFinder.status = append(invFinder.status, status)
	return invFinder
}

func (invFinder *invoiceFinder) FromAt(start time.Time) FinderInvoice {
	invFinder.start = start
	return invFinder
}

func (invFinder *invoiceFinder) ToAt(end time.Time) FinderInvoice {

	invFinder.end = end

	return invFinder
}

func (invFinder *invoiceFinder) Find(ctx context.Context, skip, limit uint) ([]*Invoice, error) {

	invoices, err := invFinder.database.Invoice().Find(ctx, skip, limit, invFinder.getRequest())

	if err != nil {
		return nil, err
	}

	var result = make([]*Invoice, 0)

	for _, inv := range invoices {
		var invoice Invoice
		invoice.fromModel(inv)
		result = append(result, &invoice)
	}

	return result, nil
}

func (invFinder *invoiceFinder) Count(ctx context.Context) (int, error) {
	return invFinder.database.Invoice().Count(ctx, &model.RequestSearchInvoice{
		Status: invFinder.status,
	})
}

func (invHandler *invoicerHandler) InvoiceFinder() FinderInvoice {
	return &invoiceFinder{database: invHandler.database}
}

func (invHandler *invoicerHandler) SendInvoice(ctx context.Context, invoice *Invoice) error {
	return nil
}
