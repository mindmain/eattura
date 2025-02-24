package eattura

import (
	"cmp"
	"context"
	"errors"

	"github.com/mindmain/eattura/core"
	"github.com/mindmain/eattura/db"
	"github.com/mindmain/eattura/db/model"
	"github.com/mindmain/eattura/sdi/fe"
)

type IssuerHandler interface {
	GetIssuer(ctx context.Context, uuid string) (*Issuer, error)
	ListIssuers(ctx context.Context) ([]*Issuer, error)

	CreateIssuer(ctx context.Context, issuer *Issuer) (*Issuer, error)
	UpdateIssuer(ctx context.Context, uuid string, issuer *Issuer) (*Issuer, error)
	DeleteIssuer(ctx context.Context, uuid string) error
}

func (i *Issuer) fromModel(raw *model.Issuer) {
	i.Name = raw.Contact.Name
	i.Surname = raw.Contact.Surname
	i.Denomination = raw.Contact.Denomination
	i.Email = raw.Contact.Email
	i.FiscalCode = raw.Contact.FiscalCode
	i.Phone = raw.Contact.Phone
	i.VatCode = &VatCode{
		Code:   raw.Contact.VatNumber,
		Nation: cmp.Or(raw.Contact.VatNation, "IT"),
	}
	i.Address = &Address{
		Country:  raw.Contact.Country,
		Province: raw.Contact.Province,
		City:     raw.Contact.City,
		Street:   raw.Contact.Street,
		Number:   raw.Contact.StreetNumber,
		ZipCode:  raw.Contact.ZipCode,
	}

	i.Rea = &Rea{
		Office:            raw.Contact.ReaOffice,
		Number:            raw.Contact.ReaNumber,
		Capital:           raw.Contact.ReaCapital,
		IsSoleShareholder: raw.Contact.ReaShareholder == fe.SU,
		InLiquidation:     raw.Contact.ReaLiquidation == fe.LS,
	}
	i.ID = raw.UUID
}

func fromConfig() *Issuer {
	var iss = &Issuer{
		ContactData: ContactData{
			Name:         core.Get("issuer.name"),
			Surname:      core.Get("issuer.surname"),
			Denomination: core.Get("issuer.denomination"),

			FiscalCode: core.Get("issuer.fiscal_code"),
			VatCode: &VatCode{
				Code:   core.Get("issuer.vat_code"),
				Nation: core.Get("issuer.vat_nation"),
			},

			Email: core.Get("issuer.email"),
			Phone: core.Get("issuer.phone"),

			Address: &Address{
				Country:  core.Get("issuer.country"),
				Province: core.Get("issuer.province"),
				City:     core.Get("issuer.city"),
				Street:   core.Get("issuer.street"),
				Number:   core.Get("issuer.number"),
				ZipCode:  core.Get("issuer.zip_code"),
			},
		},
	}

	if core.Get("issuer.rea.capital") != "" {
		capital := core.GetFloat("issuer.rea.capital")
		iss.Rea = &Rea{
			Office:            core.Get("issuer.rea.office"),
			Number:            core.Get("issuer.rea.number"),
			Capital:           capital,
			IsSoleShareholder: core.GetBool("issuer.rea.shareholder"),
			InLiquidation:     core.GetBool("issuer.rea.liquidation"),
		}
	}

	return iss
}

type issuerHandler struct {
	db   db.Database
	cred handlerCredential
}

func (i *issuerHandler) GetIssuer(ctx context.Context, uuid string) (*Issuer, error) {

	if core.IsIssuerStatic() {
		return fromConfig(), nil
	}

	if core.IsIssuerDB() {

		issuer, err := i.db.Issuer().Read(ctx, uuid)
		if err != nil {
			return nil, err
		}

		var result = &Issuer{}
		result.fromModel(issuer)
		return result, nil
	}

	return nil, errors.New("config issuer type unknown")
}

func (i *issuerHandler) ListIssuers(ctx context.Context) ([]*Issuer, error) {

	if core.IsIssuerStatic() {
		return []*Issuer{fromConfig()}, nil
	}

	if core.IsIssuerDB() {

		issuers, err := i.db.Issuer().FindAll(ctx)
		if err != nil {
			return nil, err
		}

		var result []*Issuer
		for _, issuer := range issuers {
			var i = &Issuer{}
			i.fromModel(issuer)
			result = append(result, i)
		}

		return result, nil
	}

	return nil, errors.New("config issuer type unknown")
}

func (i *issuerHandler) CreateIssuer(ctx context.Context, issuer *Issuer) (*Issuer, error) {

	if core.IsIssuerStatic() {
		return nil, errors.New("issuer is static")
	}

	if core.IsIssuerDB() {

		if issuer.VatCode == nil {
			return nil, errors.New("vat code is required")
		}

		if issuer.Address == nil {
			return nil, errors.New("address is required")
		}

		if issuer.VatCode.Code == "" || issuer.VatCode.Nation == "" {
			return nil, errors.New("vat code is required")
		}

		issuer.ID = determinateUUID(issuer.VatCode)

		if issuer.CredentialID == "" {
			return nil, errors.New("credential is required")
		}

		creds, err := i.cred.GetCredential(ctx, issuer.CredentialID)

		if err != nil {
			return nil, err
		}

		issuerModel := issuer.toModel()

		issuerModel.CredentialUUID = creds.ID

		err = i.db.Issuer().Create(ctx, issuerModel)

		if err != nil {
			return nil, err
		}

		result, err := i.db.Issuer().Read(ctx, issuerModel.UUID)

		if err != nil {
			return nil, err
		}

		issuer.fromModel(result)

		return issuer, nil
	}
	return issuer, nil
}
func (i *issuerHandler) UpdateIssuer(ctx context.Context, uuid string, issuer *Issuer) (*Issuer, error) {

	if core.IsIssuerStatic() {
		return nil, errors.New("issuer is static")
	}

	if core.IsIssuerDB() {

		if issuer.VatCode == nil {
			return nil, errors.New("vat code is required")
		}

		if issuer.Address == nil {
			return nil, errors.New("address is required")
		}

		if issuer.VatCode.Code == "" || issuer.VatCode.Nation == "" {
			return nil, errors.New("vat code is required")
		}

		issuerModel, err := i.db.Issuer().Read(ctx, uuid)
		if err != nil {
			return nil, err
		}

		if issuer.CredentialID == "" {
			return nil, errors.New("credential is required")
		}

		creds, err := i.cred.GetCredential(ctx, issuer.CredentialID)

		if err != nil {
			return nil, err
		}

		issuerModel.CredentialUUID = creds.ID

		err = i.db.Issuer().Update(ctx, uuid, issuerModel)
		if err != nil {
			return nil, err
		}

		result, err := i.db.Issuer().Read(ctx, uuid)
		if err != nil {
			return nil, err
		}
		var up = &Issuer{}
		up.fromModel(result)

		return up, nil
	}

	return nil, nil

}
func (i *issuerHandler) DeleteIssuer(ctx context.Context, uuid string) error {

	if core.IsIssuerStatic() {
		return errors.New("issuer is static")
	}

	if core.IsIssuerDB() {

		_, err := i.cred.DeleteCredential(ctx, uuid)
		if err != nil {
			return err
		}

		err = i.db.Issuer().Delete(ctx, uuid)
		if err != nil {
			return err
		}

	}

	return nil

}
