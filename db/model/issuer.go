package model

import "time"

type Issuer struct {
	UUID           string       `json:"uuid" gorm:"primaryKey;column:uuid"`
	CredentialUUID string       `json:"credential_uuid" gorm:"index;column:credential_uuid"`
	Credential     *Credentials `json:"credential" gorm:"foreignKey:CredentialUUID;references:UUID"`

	VatId      string `json:"vat_id" gorm:"type:text;index;column:vat_id"`
	FiscalCode string `json:"fiscal_code" gorm:"size:255;column:fiscal_code"`

	Title        string `json:"title" gorm:"size:255;column:title"`
	Name         string `json:"name" gorm:"size:255;column:name"`
	Surname      string `json:"surname" gorm:"size:255;column:surname"`
	Denomination string `json:"denomination" gorm:"size:255;column:denomination"`

	Country      string `json:"country" gorm:"size:255;column:country"`
	Province     string `json:"province" gorm:"size:255;column:province"`
	City         string `json:"city" gorm:"size:255;column:city"`
	Street       string `json:"street" gorm:"type:text;column:street"`
	StreetNumber string `json:"street_number" gorm:"size:10;column:street_number"`
	ZipCode      string `json:"zip_code" gorm:"size:10;column:zip_code"`

	Phone string `json:"phone" gorm:"size:20;column:phone"`
	Email string `json:"email" gorm:"size:255;column:email"`

	ReaOffice      string  `json:"rea_office" gorm:"size:255;column:rea_office"`
	ReaNumber      string  `json:"rea_number" gorm:"size:255;column:rea_number"`
	ReaCapital     float64 `json:"rea_social_capital" gorm:"column:rea_social_capital"`
	ReaShareholder string  `json:"rea_shareholder" gorm:"size:2;column:rea_shareholder"`
	ReaLiquidation string  `json:"rea_liquidation" gorm:"size:2;column:rea_liquidation"`

	CodEORI string `json:"cod_eori" gorm:"size:20;column:cod_eori"`

	CreatedAt time.Time `json:"created_at" gorm:"autoCreateTime;column:created_at"`
	UpdatedAt time.Time `json:"updated_at" gorm:"autoUpdateTime;column:updated_at"`
}

func (i *Issuer) TableName() string {
	return "issuers"
}
