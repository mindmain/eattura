package model

import "time"

type BillingType string

const (
	// A company can be a freelancer or a company with a `p.iva`.
	BillingTypeCompany BillingType = "company"
	// Is a subject hasn't been `p.iva` registered, it's considered a customer.
	BillingTypePerson BillingType = "person"
)

type BillingRole string

const (
	BillingRoleCustomer BillingRole = "customer"
	BillingRoleSupplier BillingRole = "supplier"
)

type DataInfo struct {
}

type Contact struct {
	UUID string ` gorm:"primaryKey;column:uuid"`

	Title        string ` gorm:"type:text;column:title"`
	Denomination string ` gorm:"type:text;column:denomination"`
	Name         string ` gorm:"type:text;column:name"`
	Surname      string ` gorm:"type:text;column:surname"`

	FiscalCode string ` gorm:"type:text;column:fiscal_code"`
	VatNumber  string ` gorm:"type:text;column:vat_number"`
	VatNation  string ` gorm:"size:2;column:vat_nation"`

	Country      string ` gorm:"size:2;column:country"`
	City         string ` gorm:"type:text;column:city"`
	ZipCode      string ` gorm:"type:text;column:zip_code"`
	Province     string ` gorm:"type:text;column:province"`
	Street       string ` gorm:"type:text;column:street"`
	StreetNumber string ` gorm:"type:text;column:street_number"`

	Email string ` gorm:"type:text;column:email"`
	Phone string ` gorm:"type:text;column:phone"`

	ReaOffice      string  `gorm:"size:255;column:rea_office"`
	ReaNumber      string  `gorm:"size:255;column:rea_number"`
	ReaCapital     float64 `gorm:"column:rea_social_capital"`
	ReaShareholder string  `gorm:"size:2;column:rea_shareholder"`
	ReaLiquidation string  `gorm:"size:2;column:rea_liquidation"`

	CodEORI string `gorm:"size:20;column:cod_eori"`

	Pec             string ` gorm:"type:text;column:pec"`
	DestinationCode string ` gorm:"size:7;column:code"`

	CreatedAt time.Time ` gorm:"column:created_at;autoCreateTime"`
	UpdatedAt time.Time ` gorm:"column:updated_at;autoUpdateTime"`
}

func (c *Contact) TableName() string {
	return "contacts"
}

type RequestSearchContact struct {
	Text string

	Offset int
	Limit  int
}

type ResponseSearchContact struct {
	Contacts []*Contact
	Total    int
	Offset   int
	Limit    int
}
