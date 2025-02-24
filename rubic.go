package eattura

import (
	"context"

	"github.com/mindmain/eattura/db"
	"github.com/mindmain/eattura/db/model"
)

type FinderContact interface {
	WithText(text string) FinderContact
	Ids(ids ...string) FinderContact
	Count(ctx context.Context) (int, error)
	List(ctx context.Context, offset, limit int) ([]*Contact, error)
}

type RequestCreateContact struct {
	ContactData     ContactData `json:"data"`
	Pec             string      `json:"pec"`
	DestinationCode string      `json:"destination_code"`
	Rea             *Rea        `json:"rea"`
}

type RequestUpdateContact struct {
	ContactData     *ContactData `json:"data"`
	Pec             string       `json:"pec"`
	DestinationCode string       `json:"destination_code"`
	Rea             *Rea         `json:"rea"`
}

type Rubric interface {
	CreateContact(ctx context.Context, contact *RequestCreateContact) (*Contact, error)
	ReadContact(ctx context.Context, uuid string) (*Contact, error)
	UpdateContact(ctx context.Context, uuid string, contact *RequestUpdateContact) (*ResponseUpdate[Contact], error)
	DeleteContact(ctx context.Context, uuid string) (*Contact, error)

	FinderContact() FinderContact
}

type contactsHandler struct {
	database db.Database
}

func (contacts *contactsHandler) CreateContact(ctx context.Context, request *RequestCreateContact) (*Contact, error) {

	if request == nil {
		return nil, ErrNilRequest
	}

	if request.ContactData.VatCode == nil {
		return nil, ErrNilRequest
	}

	contactModel := request.ContactData.toModel()

	contactModel.Pec = request.Pec
	contactModel.DestinationCode = request.DestinationCode

	if request.Rea != nil {
		request.Rea.toModel(contactModel)
	}

	contactModel.UUID = determinateUUID(request.ContactData.VatCode)

	err := contacts.database.Contact().Create(ctx, contactModel)

	if err != nil {
		return nil, err
	}

	result, err := contacts.database.Contact().Read(ctx, contactModel.UUID)

	if err != nil {
		return nil, err
	}

	var contact = &Contact{}
	contact.fromModel(result)
	return contact, nil
}

func (contacts *contactsHandler) ReadContact(ctx context.Context, uuid string) (*Contact, error) {

	if uuid == "" {
		return nil, ErrNilRequest
	}

	result, err := contacts.database.Contact().Read(ctx, uuid)

	if err != nil {
		return nil, err
	}

	var contact = &Contact{}
	contact.fromModel(result)
	return contact, nil

}
func (contacts *contactsHandler) UpdateContact(ctx context.Context, uuid string, request *RequestUpdateContact) (*ResponseUpdate[Contact], error) {

	if request == nil {
		return nil, ErrNilRequest
	}

	if request.ContactData == nil {
		return nil, ErrNilRequest
	}

	if request.ContactData.VatCode == nil {
		return nil, ErrNilRequest
	}

	contactModelOld, err := contacts.database.Contact().Read(ctx, uuid)

	if err != nil {
		return nil, err
	}
	contactOld := &Contact{}
	contactOld.fromModel(contactModelOld)

	contactModel := request.ContactData.toModel()

	contactModel.Pec = request.Pec
	contactModel.DestinationCode = request.DestinationCode

	if request.Rea != nil {
		request.Rea.toModel(contactModel)
	}

	if request.ContactData.VatCode.String() != contactOld.VatCode.String() {
		return nil, ErrCannotChangeContactVatCode
	}

	err = contacts.database.Contact().Update(ctx, uuid, contactModel)

	if err != nil {
		return nil, err
	}

	result, err := contacts.database.Contact().Read(ctx, uuid)

	if err != nil {
		return nil, err
	}

	var contact = &Contact{}
	contact.fromModel(result)
	return &ResponseUpdate[Contact]{
		Old: contactOld,
		New: contact,
	}, nil

}
func (contacts *contactsHandler) DeleteContact(ctx context.Context, uuid string) (*Contact, error) {

	countInvoice, err := contacts.database.Invoice().Count(ctx, &model.RequestSearchInvoice{
		ContactUUID: []string{uuid},
	})

	if err != nil {
		return nil, err
	}

	if countInvoice > 0 {
		return nil, ErrCannotDeleteContactWithInvoice
	}

	result, err := contacts.database.Contact().Read(ctx, uuid)

	if err != nil {
		return nil, err
	}

	err = contacts.database.Contact().Delete(ctx, uuid)

	if err != nil {
		return nil, err
	}

	var contact = &Contact{}
	contact.fromModel(result)

	return contact, nil
}

type finderContact struct {
	database db.Database
	text     string
	ids      []string
}

func (finder *finderContact) WithText(text string) FinderContact {
	finder.text = text
	return finder
}

func (finder *finderContact) Ids(ids ...string) FinderContact {
	finder.ids = ids
	return finder
}

func (finder *finderContact) Count(ctx context.Context) (int, error) {
	return finder.database.Contact().Count(ctx, &model.RequestSearchContact{
		Text: finder.text,
		Ids:  finder.ids,
	})
}

func (finder *finderContact) List(ctx context.Context, offset, limit int) ([]*Contact, error) {
	contacts, err := finder.database.Contact().Find(ctx, uint(offset), uint(limit), &model.RequestSearchContact{
		Text: finder.text,
		Ids:  finder.ids,
	})

	if err != nil {
		return nil, err
	}

	var result []*Contact
	for _, contact := range contacts {
		customer := &Contact{}
		customer.fromModel(contact)
		result = append(result, customer)
	}

	return result, nil
}

func (contacts *contactsHandler) FinderContact() FinderContact {
	return &finderContact{
		database: contacts.database,
	}
}
