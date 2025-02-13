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
	BillingRoleBoth     BillingRole = "both"
)

type Contact struct {
	UUID string `json:"uuid" gorm:"primaryKey;column:uuid"`

	BillingType BillingType `json:"type" gorm:"index;type:VarChar(20);column:type"`
	Role        BillingRole `json:"role" gorm:"index;type:VarChar(20);column:role"`

	Denomination string `json:"denomination" gorm:"type:text;column:denomination"`
	Name         string `json:"name" gorm:"type:text;column:name"`
	Surname      string `json:"surname" gorm:"type:text;column:surname"`

	Nation     string `json:"nation" gorm:"size:2;column:nation"`
	VatNumber  string `json:"vat_number" gorm:"type:text;column:vat_number"`
	FiscalCode string `json:"fiscal_code" gorm:"type:text;column:fiscal_code"`
	Street     string `json:"street" gorm:"type:text;column:street"`
	City       string `json:"city" gorm:"type:text;column:city"`
	ZipCode    string `json:"zipCode" gorm:"type:text;column:zip_code"`
	Region     string `json:"region" gorm:"type:text;column:region"`

	Pec             string `json:"pec" gorm:"type:text;column:pec"`
	DestinationCode string `json:"code" gorm:"size:7;column:code"`

	Email string `json:"email" gorm:"type:text;column:email"`
	Phone string `json:"phone" gorm:"type:text;column:phone"`

	CreatedAt time.Time `json:"created_at" gorm:"column:created_at;autoCreateTime"`
	UpdatedAt time.Time `json:"updated_at" gorm:"column:updated_at;autoUpdateTime"`
}

func (c *Contact) TableName() string {
	return "contacts"
}

type RequestSearchContact struct {
	Role        []BillingRole `json:"role"`
	BillingType []BillingType `json:"billing_type"`
	Text        string        `json:"text"`

	Offset int `json:"offset"`
	Limit  int `json:"limit"`
}

type ResponseSearchContact struct {
	Contacts []*Contact
	Total    int `json:"total"`

	Offset int `json:"offset"`
	Limit  int `json:"limit"`
}
