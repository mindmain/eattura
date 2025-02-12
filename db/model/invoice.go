package model

import (
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

type InvoiceItem struct {
	InvoiceUUID string  `json:"invoice_uuid" xorm:"pk 'invoice_uuid'"`
	Description string  `json:"description" xorm:"TEXT 'description'"`
	Nature      string  `json:"nature" xorm:"varChar(10) 'nature'"`
	Amount      float64 `json:"amount" xorm:"DECIMAL(10,2) 'amount'"`
	Vat         float64 `json:"vat" xorm:"DECIMAL(10,2) 'vat'"`
	Total       float64 `json:"total" xorm:"DECIMAL(10,2) 'total'"`
}

type Invoice struct {
	UUID   string `json:"uuid" xorm:"pk 'uuid'"`
	Status Status `json:"status" xorm:"varChar(20) 'status'"`

	InvoiceNumber string `json:"number" xorm:"varChar(20) 'invoice_number'"`
	TypeDocument  string `json:"type_document" xorm:"varChar(10) 'type_document'"`
	File          string `json:"file" xorm:"TEXT 'file'"`

	IssuerUUID string  `json:"issuer_uuid" xorm:"index 'issuer_uuid'"`
	Issuer     *Issuer `json:"issuer" xorm:"-"`

	Contact     *Contact   `json:"contact" xorm:"-"`
	ContactUUID string     `json:"contact_uuid" xorm:"index 'contact_uuid'"`
	Hash        string     `json:"hash" xorm:"TEXT 'hash'"`
	Date        *time.Time `json:"date" xorm:"DATE 'date'"`

	Items         []*InvoiceItem  `json:"items" xorm:"-"`
	Notifications []*Notification `json:"notifications" xorm:"-"`

	AmountPaid float64 `json:"amount_paid" xorm:"DECIMAL(10,2) 'amount_paid'"`

	UniqueProgressive string    `json:"unique_progressive" xorm:"VarChar(5) index unique 'unique_progressive'"`
	ProgressiveNumber string    `json:"progressive_number" xorm:"VarChar(10) index unique 'progressive_number'"`
	SentAt            time.Time `json:"sent_at" xorm:"DATETIME 'sent_at'"`
	CreatedAt         time.Time `json:"created_at" xorm:"created 'created_at'"`
	UpdatedAt         time.Time `json:"updated_at" xorm:"updated 'updated_at'"`
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
	InvoiceUUID string           `json:"invoice_uuid" xorm:"pk 'invoice_uuid'"`
	At          time.Time        `json:"at" xorm:"DATETIME 'at'"`
	Type        NotificationType `json:"type" xorm:"varChar(10) 'type'"`
	File        string           `json:"filename" xorm:"TEXT 'file'"`
	CreatedAt   time.Time        `json:"created_at" xorm:"created 'created_at'"`
}

type RequestSearchInvoice struct {
	ContactUUID []string `json:"contact_uuid"`
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
