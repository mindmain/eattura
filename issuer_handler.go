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
}

func (i *Issuer) fromModel(raw *model.Issuer) {
	i.Name = raw.Name
	i.Surname = raw.Surname
	i.Denomination = raw.Denomination
	i.Email = raw.Email
	i.FiscalCode = raw.FiscalCode
	i.Phone = raw.Phone
	i.VatCode = &VatCode{
		Code:   raw.VatId,
		Nation: cmp.Or(raw.VatNation, "IT"),
	}
	i.Address = &Address{
		Country:  raw.Country,
		Province: raw.Province,
		City:     raw.City,
		Street:   raw.Street,
		Number:   raw.StreetNumber,
		ZipCode:  raw.ZipCode,
	}

	i.Rea = &Rea{
		Office:            raw.ReaOffice,
		Number:            raw.ReaNumber,
		Capital:           raw.ReaCapital,
		IsSoleShareholder: raw.ReaShareholder == fe.SU,
		InLiquidation:     raw.ReaLiquidation == fe.LS,
	}

	i.uuidCredential = raw.CredentialUUID
}

func fromConfig() *Issuer {
	var iss = &Issuer{
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
	db db.Database
}

func (i *issuerHandler) GetIssuer(ctx context.Context, uuid string) (*Issuer, error) {

	checkType := core.Get("issuer.type")
	if checkType == "static" {
		return fromConfig(), nil
	}

	if checkType != "db" {

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
