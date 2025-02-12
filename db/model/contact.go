package model

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
	UUID string `json:"uuid" xorm:"pk 'uuid'"`

	BillingType BillingType `json:"type" xorm:"index VarChar(20) 'type'"`
	Role        BillingRole `json:"role" xorm:"index VarChar(20) 'role'"`

	Denomination string `json:"denomination" xorm:"TEXT 'denomination'"`
	Name         string `json:"name" xorm:"TEXT 'name'"`
	Surname      string `json:"surname" xorm:"TEXT 'surname'"`

	Nation     string `json:"nation" xorm:"VarChar(2) 'nation'"`
	VatNumber  string `json:"vat_number" xorm:"TEXT 'vat_number'"`
	FiscalCode string `json:"fiscal_code" xorm:"TEXT 'fiscal_code'"`
	Street     string `json:"street" xorm:"TEXT 'street'"`
	City       string `json:"city"  xorm:"TEXT 'city'"`
	ZipCode    string `json:"zipCode"  xorm:"TEXT 'zip_code'"`
	Region     string `json:"region" xorm:"TEXT 'region'"`

	Pec             string `json:"pec" xorm:"TEXT 'pec'"`
	DestinationCode string `json:"code" xorm:"VarChar(7) 'code'"`

	Email string `json:"email" xorm:"TEXT 'email'"`
	Phone string `json:"phone" xorm:"TEXT 'phone'"`

	CreatedAt string `json:"created_at" xorm:"created 'created_at'"`
	UpdatedAt string `json:"updated_at" xorm:"updated 'updated_at'"`
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
