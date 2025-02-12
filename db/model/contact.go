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
	UUID string `json:"uuid" xorm:"pk"`

	BillingType BillingType `json:"billing_type" xorm:"index,VarChar(20)"`
	Role        BillingRole `json:"role" xorm:"index,VarChar(20)"`

	Denomination string `json:"denomination" xorm:"TEXT"`
	Name         string `json:"name" xorm:"TEXT"`
	Surname      string `json:"surname" xorm:"TEXT"`

	Nation     string `json:"nation" xorm:"VarChar(2)"`
	VatNumber  string `json:"vat_number" xorm:"TEXT"`
	FiscalCode string `json:"fiscal_code" xorm:"TEXT"`
	Street     string `json:"street" xorm:"TEXT"`
	City       string `json:"city"  xorm:"TEXT"`
	ZipCode    string `json:"zipCode"  xorm:"TEXT"`
	Region     string `json:"region" xorm:"TEXT" `

	Pec             string `json:"pec" xorm:"TEXT"`
	DestinationCode string `json:"destination_code" xorm:"VarChar(7)"`

	Email string `json:"email" xorm:"TEXT"`
	Phone string `json:"phone" xorm:"TEXT"`

	CreatedAt string `json:"created_at" xorm:"created"`
	UpdatedAt string `json:"updated_at" xorm:"updated"`
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
