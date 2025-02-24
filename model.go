package eattura

import (
	"github.com/mindmain/eattura/db/model"
	"github.com/mindmain/eattura/sdi/fe"
)

func (c *Contact) Supplier() *Supplier {

	return &Supplier{
		ID:          c.ID,
		ContactData: c.ContactData,
		Rea:         c.Rea,
	}
}

func (c *Contact) Customer() *Customer {

	return &Customer{
		ID:              c.ID,
		ContactData:     c.ContactData,
		Pec:             c.Pec,
		DestinationCode: c.DestinationCode,
	}
}

func (c *Contact) fromModel(in *model.Contact) {
	c.ID = in.UUID
	c.ContactData.fromModel(in)
	c.DestinationCode = in.DestinationCode
	c.Pec = in.Pec

	if c.Rea == nil {
		c.Rea = &Rea{}
	}

	c.Rea.fromModel(in)
}

func (rea *Rea) toModel(contact *model.Contact) {
	contact.ReaOffice = rea.Office
	contact.ReaNumber = rea.Number
	contact.ReaCapital = rea.Capital

	if rea.InLiquidation {
		contact.ReaLiquidation = fe.LS
	} else {
		contact.ReaLiquidation = fe.LN
	}

	if rea.IsSoleShareholder {
		contact.ReaShareholder = fe.SU
	} else {
		contact.ReaShareholder = fe.SM
	}
}

func (rea *Rea) fromModel(contact *model.Contact) {
	rea.Office = contact.ReaOffice

	rea.Number = contact.ReaNumber
	rea.Capital = contact.ReaCapital

	rea.InLiquidation = contact.ReaLiquidation == fe.LS
	rea.IsSoleShareholder = contact.ReaShareholder == fe.SU
}

func (c *ContactData) toModel() *model.Contact {
	return &model.Contact{

		Title:        c.Title,
		Name:         c.Name,
		Surname:      c.Surname,
		Denomination: c.Denomination,

		FiscalCode: c.FiscalCode,
		VatNation:  c.VatCode.Nation,
		VatNumber:  c.VatCode.Code,

		Country:      c.Address.Country,
		City:         c.Address.City,
		Street:       c.Address.Street,
		StreetNumber: c.Address.Number,
		Province:     c.Address.Province,
		ZipCode:      c.Address.ZipCode,

		Email: c.Email,
		Phone: c.Phone,
	}
}

func (contact *ContactData) fromModel(in *model.Contact) {
	contact.Title = in.Title
	contact.Name = in.Name
	contact.Surname = in.Surname
	contact.Denomination = in.Denomination

	contact.FiscalCode = in.FiscalCode
	contact.VatCode.Code = in.VatNumber
	contact.VatCode.Nation = in.VatNation

	contact.Address.Country = in.Country
	contact.Address.City = in.City
	contact.Address.Street = in.Street
	contact.Address.Number = in.StreetNumber
	contact.Address.Province = in.Province
	contact.Address.ZipCode = in.ZipCode

	contact.Email = in.Email
	contact.Phone = in.Phone
}

func (issuer *Issuer) toModel() *model.Issuer {

	contactModel := issuer.ContactData.toModel()

	issuer.Rea.toModel(contactModel)

	return &model.Issuer{
		UUID:             issuer.ID,
		ContactReference: issuer.ID,
		Contact:          contactModel,
		CredentialUUID:   issuer.CredentialID,
	}
}

func (inv *Invoice) toModel() *model.Invoice {

	var mod = &model.Invoice{
		UUID:            inv.ID,
		Status:          model.Status(inv.Status),
		TypeDocument:    string(inv.Type),
		IssuerReference: inv.Issuer.ID,
		Issuer:          inv.Issuer.toModel(),
		InvoiceNumber:   inv.Number,
		Date:            inv.Date,

		Items: make([]*model.InvoiceItem, 0),
	}

	for _, item := range inv.Items {
		mod.Items = append(mod.Items, &model.InvoiceItem{
			UUID:        item.ID,
			InvoiceUUID: inv.ID,
			Description: item.Description,
			Nature:      string(item.Nature),
			UnitAmount:  item.UnitAmount,
			Vat:         item.VatAmount,
			Total:       item.Total(),
			Qty:         item.Qty,
			Unit:        item.Unit,
		})
	}
	return mod
}

func (i *Invoice) fromModel(in *model.Invoice) {

	i.ID = in.UUID
	i.Status = in.Status
	i.Type = TypeDocument(in.TypeDocument)
	i.Number = in.InvoiceNumber
	i.Date = in.Date

	i.Issuer = &Issuer{}
	i.Issuer.fromModel(in.Issuer)

	i.Items = make([]*InvoiceItem, 0, len(in.Items))

	for _, item := range in.Items {
		i.Items = append(i.Items, &InvoiceItem{
			ID:          item.UUID,
			Description: item.Description,
			Nature:      Natura(item.Nature),
			UnitAmount:  item.UnitAmount,
			VatAmount:   item.Vat,
			Qty:         item.Qty,
			Unit:        item.Unit,
		})
	}

	i.Supplier = &Supplier{}
	i.Supplier.fromModel(in.Supplier)

	i.Customer = &Customer{}
	i.Customer.fromModel(in.Customer)

}
