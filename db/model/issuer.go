package model

import "time"

type Issuer struct {
	UUID           string       `json:"uuid" xorm:"pk 'uuid'"`
	CredentialUUID string       `json:"credential_uuid" xorm:"index 'credential_uuid'"`
	Credential     *Credentials `json:"credential" xorm:"-"`
	VatId          string       `json:"vat_id" xorm:"TEXT index 'vat_id'"`
	Name           string       `json:"name" xorm:"VarChar(255) 'name'"`
	Surname        string       `json:"surname" xorm:"VarChar(255) 'surname'"`
	Denomination   string       `json:"denomination" xorm:"VarChar(255) 'denomination'"`
	FiscalCode     string       `json:"fiscal_code" xorm:"VarChar(255) 'fiscal_code'"`
	Street         string       `json:"street" xorm:"TEXT 'street'"`
	City           string       `json:"city" xorm:"VarChar(255) 'city'"`
	ZipCode        string       `json:"zip_code" xorm:"VarChar(10) 'zip_code'"`
	Region         string       `json:"region" xorm:"VarChar(255) 'region'"`
	Country        string       `json:"country" xorm:"VarChar(255) 'country'"`
	CreatedAt      time.Time    `json:"created_at" xorm:"created 'created_at'"`
	UpdatedAt      time.Time    `json:"updated_at" xorm:"updated 'updated_at'"`
}

func (i *Issuer) TableName() string {
	return "issuers"
}
