package model

import (
	"fmt"
	"time"
)

type Status string

const (
	// Eattura has been created, but not sent to pec yet, user can still modify it.
	StatusDraft Status = "draft"
	// Eattura has been created, waiting to be sent to pec.
	StatusPending Status = "pending"
	// Eattura has been sent to pec, waiting for response.
	StatusSent Status = "sent"
	// SDI response from pec, invoice is rejected, something went wrong.
	StatusFailed Status = "fail"
	// SDI response from pec, invoice is accepted.
	StatusSuccess Status = "success"

	//User indicates that the invoice has been paid partially, in the amount of AmountPaid.
	StatusPartialPaid Status = "partial-paid"
	// User indicates that the invoice has been paid in full.
	StatusPaid Status = "paid"

	// Is the status when the invoice is received from the SDI.
	StatusReceived Status = "received"
)

func (s Status) String() string {
	return string(s)
}

type InvoiceItem struct {
	UUID        string  `gorm:"primaryKey;column:uuid"`
	InvoiceUUID string  `gorm:"column:invoice_uuid;index"`
	Description string  `gorm:"type:TEXT;column:description;index"`
	Nature      string  `gorm:"size:10;column:nature"`
	UnitAmount  float64 `gorm:"type:DECIMAL(10,2);column:unit_amount"`
	Qty         float64 `gorm:"type:DECIMAL(10,2);column:qty"`
	Vat         float64 `gorm:"type:DECIMAL(10,2);column:vat"`
	Unit        string  `gorm:"size:10;column:unit"`
	Total       float64 `gorm:"type:DECIMAL(10,2);column:total"`
}

func (i *InvoiceItem) TableName() string {
	return "invoice_items"
}

type Invoice struct {
	UUID   string `gorm:"primaryKey;column:uuid"`
	Status Status `gorm:"size:20;column:status"`

	InvoiceNumber string `gorm:"size:20;column:invoice_number"`
	TypeDocument  string `gorm:"type:varChar(10);column:type_document"`
	File          string `gorm:"type:TEXT;column:file"`

	IssuerReference string  `gorm:"index;column:issuer_uuid"`
	Issuer          *Issuer `gorm:"foreignKey:UUID;references:IssuerReference"`

	CustomerReference string   `gorm:"index;column:customer_uuid;default:null"`
	Customer          *Contact `gorm:"foreignKey:UUID;references:CustomerReference"`

	SupplierReference string   `gorm:"index;column:supplier_uuid;default:null"`
	Supplier          *Contact `gorm:"foreignKey:UUID;references:SupplierReference"`

	Hash string    `gorm:"type:TEXT;column:hash"`
	Date time.Time `gorm:"column:date;index"`

	Items         []*InvoiceItem  `gorm:"foreignKey:InvoiceUUID;references:UUID"`
	Notifications []*Notification `gorm:"foreignKey:InvoiceUUID;references:UUID"`

	AmountPaid float64 `gorm:"type:DECIMAL(10,2);column:amount_paid"`

	UniqueProgressive string `gorm:"size:5;uniqueIndex;column:unique_progressive"`
	ProgressiveNumber string `gorm:"size:10;uniqueIndex;column:progressive_number"`

	SentAt    time.Time `gorm:"column:sent_at;default:null"`
	CreatedAt time.Time `gorm:"autoCreateTime;column:created_at"`
	UpdatedAt time.Time `gorm:"autoUpdateTime;column:updated_at"`
}

func (i *Invoice) String() string {
	return fmt.Sprintf("[invoice: %s from %s to %s %d ]\n", i.UUID, i.SupplierReference, i.CustomerReference, len(i.Items))
}

func (i *Invoice) TableName() string {
	return "invoices"
}

type NotificationType string

const (
	Success NotificationType = "success"
	Fail    NotificationType = "fail"
)

// when SDI  form pec result of the invoice, it will be saved in the notification.
// an invoice can have multiple notifications.
// in case of success, the file will be saved 1 notification.
// in case of fail, the file will be saved 2 or more notifications.
type Notification struct {
	InvoiceUUID string           `gorm:"primaryKey;column:invoice_uuid"`
	At          time.Time        `gorm:"type:DATETIME;column:at"`
	Type        NotificationType `gorm:"size:10;column:type"`
	File        string           `gorm:"type:TEXT;column:file"`
	CreatedAt   time.Time        `gorm:"autoCreateTime;column:created_at"`
}

type RequestSearchInvoice struct {
	ContactUUID  []string
	IssuerUUID   []string
	SupplierUUID []string
	Status       []Status

	StartDate *time.Time
	EndDate   *time.Time
	Text      string

	Offset int
	Limit  int
}

type ResponseSearchInvoice struct {
	Invoices []*Invoice
	Total    int

	Offset int
	Limit  int
}

type RequestSearchInvoiceItem struct {
	InvoiceUUID []string
	Text        string

	Offset int
	Limit  int
}

type ResponseSearchInvoiceItem struct {
	Items []*InvoiceItem
	Total int

	Offset int
	Limit  int
}
