package model

import "time"

type Issuer struct {
	UUID           string       `json:"uuid" xorm:"pk"`
	CredentialUUID string       `json:"credential_uuid" xorm:"index"`
	Credential     *Credentials `json:"credential" xorm:"-"`
	VatId          string       `json:"vat_id" xorm:"TEXT,index"`
	Name           string       `json:"name" xorm:"VarChar(255)"`
	Surname        string       `json:"surname" xorm:"VarChar(255)"`
	Denomination   string       `json:"denomination" xorm:"VarChar(255)"`
	FiscalCode     string       `json:"fiscal_code" xorm:"VarChar(255)"`
	Street         string       `json:"street" xorm:"TEXT"`
	City           string       `json:"city" xorm:"VarChar(255)"`
	ZipCode        string       `json:"zip_code" xorm:"VarChar(10)"`
	Region         string       `json:"region" xorm:"VarChar(255)"`
	Country        string       `json:"country" xorm:"VarChar(255)"`
	Valid          bool         `json:"valid" xorm:"BOOL"`

	CreatedAt time.Time `json:"created_at" xorm:"created"`
	UpdatedAt time.Time `json:"updated_at" xorm:"updated"`
}

func (i *Issuer) TableName() string {
	return "issuers"
}
