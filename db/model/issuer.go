package model

import "time"

type Issuer struct {
	UUID           string       `json:"uuid" gorm:"primaryKey;column:uuid"`
	CredentialUUID string       `json:"credential_uuid" gorm:"index;column:credential_uuid"`
	Credential     *Credentials `json:"credential" gorm:"-"`
	VatId          string       `json:"vat_id" gorm:"type:text;index;column:vat_id"`
	Name           string       `json:"name" gorm:"size:255;column:name"`
	Surname        string       `json:"surname" gorm:"size:255;column:surname"`
	Denomination   string       `json:"denomination" gorm:"size:255;column:denomination"`
	FiscalCode     string       `json:"fiscal_code" gorm:"size:255;column:fiscal_code"`
	Street         string       `json:"street" gorm:"type:text;column:street"`
	City           string       `json:"city" gorm:"size:255;column:city"`
	ZipCode        string       `json:"zip_code" gorm:"size:10;column:zip_code"`
	Region         string       `json:"region" gorm:"size:255;column:region"`
	Country        string       `json:"country" gorm:"size:255;column:country"`
	CreatedAt      time.Time    `json:"created_at" gorm:"autoCreateTime;column:created_at"`
	UpdatedAt      time.Time    `json:"updated_at" gorm:"autoUpdateTime;column:updated_at"`
}

func (i *Issuer) TableName() string {
	return "issuers"
}
