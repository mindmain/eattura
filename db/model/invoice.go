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
	UUID        string  `json:"uuid" gorm:"primaryKey;column:uuid"`
	InvoiceUUID string  `json:"invoice_uuid" gorm:"column:invoice_uuid;index"`
	Description string  `json:"description" gorm:"type:TEXT;column:description;index"`
	Nature      string  `json:"nature" gorm:"size:10;column:nature"`
	Amount      float64 `json:"amount" gorm:"type:DECIMAL(10,2);column:amount"`
	Vat         float64 `json:"vat" gorm:"type:DECIMAL(10,2);column:vat"`
	Total       float64 `json:"total" gorm:"type:DECIMAL(10,2);column:total"`
}

func (i *InvoiceItem) TableName() string {
	return "invoice_items"
}

type Invoice struct {
	UUID   string `json:"uuid" gorm:"primaryKey;column:uuid"`
	Status Status `json:"status" gorm:"size:20;column:status"`

	InvoiceNumber string `json:"number" gorm:"size:20;column:invoice_number"`
	TypeDocument  string `json:"type_document" gorm:"type:varChar(10);column:type_document"`
	File          string `json:"file" gorm:"type:TEXT;column:file"`

	IssuerReference string  `json:"issuer_uuid" gorm:"index;column:issuer_uuid"`
	Issuer          *Issuer `json:"issuer" gorm:"foreignKey:UUID;references:IssuerReference"`

	ContactReference string    `json:"contact_uuid" gorm:"index;column:contact_uuid"`
	Contact          *Contact  `json:"contact" gorm:"foreignKey:UUID;references:ContactReference"`
	Hash             string    `json:"hash" gorm:"type:TEXT;column:hash"`
	Date             time.Time `json:"date" gorm:"column:date;index"`

	Items         []*InvoiceItem  `json:"items" gorm:"foreignKey:InvoiceUUID;references:UUID"`
	Notifications []*Notification `json:"notifications" gorm:"foreignKey:InvoiceUUID;references:UUID"`

	AmountPaid float64 `json:"amount_paid" gorm:"type:DECIMAL(10,2);column:amount_paid"`

	UniqueProgressive string    `json:"unique_progressive" gorm:"size:5;uniqueIndex;column:unique_progressive"`
	ProgressiveNumber string    `json:"progressive_number" gorm:"size:10;uniqueIndex;column:progressive_number"`
	SentAt            time.Time `json:"sent_at" gorm:"column:sent_at"`
	CreatedAt         time.Time `json:"created_at" gorm:"autoCreateTime;column:created_at"`
	UpdatedAt         time.Time `json:"updated_at" gorm:"autoUpdateTime;column:updated_at"`
}

func (i *Invoice) String() string {
	return fmt.Sprintf("[invoice: %s to %s %d ]\n", i.UUID, i.ContactReference, len(i.Items))
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
	InvoiceUUID string           `json:"invoice_uuid" gorm:"primaryKey;column:invoice_uuid"`
	At          time.Time        `json:"at" gorm:"type:DATETIME;column:at"`
	Type        NotificationType `json:"type" gorm:"size:10;column:type"`
	File        string           `json:"filename" gorm:"type:TEXT;column:file"`
	CreatedAt   time.Time        `json:"created_at" gorm:"autoCreateTime;column:created_at"`
}

type RequestSearchInvoice struct {
	ContactUUID []string `json:"contact_uuid"`
	IssuerUUID  []string `json:"issuer_uuid"`
	Status      []Status `json:"status"`

	StartDate *time.Time `json:"start_date"`
	EndDate   *time.Time `json:"end_date"`
	Text      string     `json:"text"`

	Offset int `json:"offset"`
	Limit  int `json:"limit"`
}

type ResponseSearchInvoice struct {
	Invoices []*Invoice `json:"invoices"`
	Total    int        `json:"total"`

	Offset int `json:"offset"`
	Limit  int `json:"limit"`
}

type RequestSearchInvoiceItem struct {
	InvoiceUUID []string `json:"invoice_uuid"`
	Text        string   `json:"text"`

	Offset int `json:"offset"`
	Limit  int `json:"limit"`
}

type ResponseSearchInvoiceItem struct {
	Items []*InvoiceItem `json:"items"`
	Total int            `json:"total"`

	Offset int `json:"offset"`
	Limit  int `json:"limit"`
}
