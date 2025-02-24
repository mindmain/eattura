package eattura

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/mindmain/eattura/db"
	"github.com/mindmain/eattura/db/model"
	"github.com/mindmain/eattura/sdi/fe"
	"github.com/mindmain/eattura/secure"

	"github.com/mindmain/eattura/fs"
	"github.com/mindmain/eattura/pec"
)

type CredentialSetting = secure.Credentials
type Status = model.Status

type Credential struct {
	ID      string             `json:"uuid"`
	Setting *CredentialSetting `json:"-"`
}

const (
	StatusDraft Status = model.StatusDraft
	StatusSent  Status = model.StatusSent

	StatusPending Status = model.StatusPending
	StatusFailed  Status = model.StatusFailed
	StatusSuccess Status = model.StatusSuccess

	StatusPartialPaid Status = model.StatusPartialPaid
	StatusPaid        Status = model.StatusPaid

	StatusReceived Status = model.StatusReceived
)

type Address struct {

	// Country is the country code, must be in ISO 3166-1 alpha-2 format.
	Country string `json:"country"`
	// Province  refer to the Italian province code, if the country is Italy, otherwise is empty. not required. can be empty.
	Province string `json:"province"`
	City     string `json:"city"`
	Street   string `json:"street"`
	Number   string `json:"number"`
	ZipCode  string `json:"zip_code"`
}

type VatCode struct {
	Code string `json:"code"`
	// Nation is the VAT code, must be in ISO 3166-1 alpha-2 format.
	Nation string `json:"nation"`
}

func (v *VatCode) String() string {
	return fmt.Sprintf("%s%s", v.Nation, v.Code)
}

// Rea is the Repertorio Economico Amministrativo, is a register of the Italian Chamber of Commerce.
// The REA number is a unique code assigned to each company registered in the Italian Chamber of Commerce.
// The REA number is used to identify the company in the Chamber of Commerce database.
type Rea struct {
	Office            string  `json:"office"`
	Number            string  `json:"number"`
	Capital           float64 `json:"capital"`
	IsSoleShareholder bool    `json:"is_sole_shareholder"`
	InLiquidation     bool    `json:"liquidation"`
}

type ContactData struct {
	Name         string `json:"name"`
	Surname      string `json:"surname"`
	Denomination string `json:"denomination"`
	Email        string `json:"email"`
	Phone        string `json:"phone"`

	VatCode    *VatCode `json:"vat_code"`
	FiscalCode string   `json:"fiscal_code"`
	// Address is the address of the customer, isn't required, but is more useful for sdi system if present.
	Address *Address `json:"address"`
	// Title is optional field, used for the title of the customer. ex. Dott. Ing. Prof.
	Title string `json:"title"`
	//(Economic Operator Registration and Identification) is a unique code assigned to economic operators and other persons registered in the European Union (EU) for customs purposes.
	CodEORI string `json:"cod_eori"`
}
type Issuer struct {
	ID          string `json:"uuid"`
	ContactData `json:",inline"`
	Rea         *Rea `json:"rea"`

	CredentialID string `json:"credential_id"`
}

type Contact struct {
	ID              string `json:"uuid"`
	Rea             *Rea   `json:"rea"`
	Pec             string `json:"pec"`
	DestinationCode string `json:"destination_code"`
	ContactData     `json:",inline"`
}

type Customer struct {
	ID              string `json:"uuid"`
	Pec             string `json:"pec"`
	DestinationCode string `json:"destination_code"`
	ContactData     `json:",inline"`
}

type Supplier struct {
	ID          string `json:"uuid"`
	ContactData `json:",inline"`
	Rea         *Rea `json:"rea,omitempty"`
}

type Natura = fe.Natura

const (
	NatureNoIvaSubject        Natura = fe.N_2
	NatureNoIvaOtherCases     Natura = fe.N_2_2
	NatureExcludedByArticle15 Natura = fe.N_1
	NatureExport              Natura = fe.N_3_1
)

type ResponseUpdate[M any] struct {
	Old *M `json:"old"`
	New *M `json:"new"`
}

type InvoiceItem struct {
	ID          string `json:"uuid"`
	Description string `json:"description"`
	// Nature determinate IVA rate, if the nature is empty, the system determinate the IVA rate 22% (or other default rate).
	Nature     Natura  `json:"nature"`
	VatAmount  float64 `json:"vat_amount"`
	UnitAmount float64 `json:"amount"`
	Unit       string  `json:"unit"`
	Qty        float64 `json:"qty"`

	Start time.Time `json:"start"`
	End   time.Time `json:"end"`
}

func (i *InvoiceItem) Total() float64 {
	return i.UnitAmount * i.Qty
}

type TypeDocument = fe.TipoDocumento

const (
	InvoiceType   TypeDocument = fe.TD01
	SimpleInvoice TypeDocument = fe.TD07
	CreditNote    TypeDocument = fe.TD04
	DebitNote     TypeDocument = fe.TD05
)

type Invoice struct {
	ID     string       `json:"uuid"`
	Type   TypeDocument `json:"type"`
	Status Status       `json:"status"`
	// If use issuer static mode on config you can omit the issuer field.
	Issuer   *Issuer   `json:"issuer"`
	Supplier *Supplier `json:"supplier"`
	Customer *Customer `json:"customer"`

	Date   time.Time `json:"date"`
	Number string    `json:"number"`

	UniqueNumber      string `json:"unique_number"`
	ProgressiveNumber string `json:"progressive_number"`

	Items []*InvoiceItem `json:"items"`
}

type RequestCreateInvoice struct {
	IssuerId      string         `json:"issuer_id"`
	CustomerId    string         `json:"customer_id"`
	Type          TypeDocument   `json:"type"`
	InvoiceNumber string         `json:"invoice_number"`
	InvoiceDate   time.Time      `json:"invoice_date"`
	Items         []*InvoiceItem `json:"items"`
}

type FinderInvoice interface {
	WithStatus(status Status) FinderInvoice

	FromAt(date time.Time) FinderInvoice
	ToAt(date time.Time) FinderInvoice

	Count(ctx context.Context) (int, error)
	Find(ctx context.Context, offset, limit uint) ([]*Invoice, error)
}

type Invoicer interface {
	CreateInvoice(ctx context.Context, request *RequestCreateInvoice) (*Invoice, error)

	// ReadInvoice read the invoice from the database and return the invoice and the fattura elettronica.
	// The fattura elettronica is the XML file that represents the invoice. and retrive from the storage.
	ReadInvoice(ctx context.Context, uuid string) (*Invoice, *fe.FatturaElettronica, error)
	UpdateInvoice(ctx context.Context, uuid string, invoice *Invoice) (*Invoice, error)
	DeleteInvoice(ctx context.Context, uuid string) error

	InvoiceFinder() FinderInvoice
	SendInvoice(ctx context.Context, inv *Invoice) error
}

var ErrInconsistentData = errors.New("inconsistent data")

type Eattura interface {
	Invoicer
	Rubric
	handlerCredential
	GeneratorProgressive
	Loader
}

func newService(
	database db.Database,
	secure secure.Storage,
	pecClient pec.Client,
	invoiceFolder fs.InvoiceFileHandler,
) Eattura {

	var gen = &defaultGeneratorProgressive{}
	var handlerCred = &handlerCredImpl{db: database, secure: secure}
	var issuerHandler = &issuerHandler{db: database, cred: handlerCred}
	return &service{
		Loader: &defaultLoader{
			db:            database,
			pec:           pecClient,
			store:         invoiceFolder,
			issuerHandler: issuerHandler,
		},
		Invoicer: &invoicerHandler{
			database:  database,
			pec:       pecClient,
			store:     invoiceFolder,
			generator: gen,
			issuer:    issuerHandler,
		},
		Rubric:               &contactsHandler{database: database},
		handlerCredential:    handlerCred,
		GeneratorProgressive: gen,
	}
}

type service struct {
	handlerCredential
	Invoicer
	Rubric
	Loader
	GeneratorProgressive
}

func New() (Eattura, error) {

	database, err := db.New()

	if err != nil {
		return nil, err
	}

	secureStorage, err := secure.New()

	if err != nil {
		return nil, err
	}

	storage, err := fs.New()

	if err != nil {
		return nil, err
	}

	pecService := pec.NewPEC(secureStorage)

	return newService(database, secureStorage, pecService, storage), nil

}
