package eattura

import (
	"context"
	"errors"
	"time"

	"github.com/mindmain/eattura/db"
	"github.com/mindmain/eattura/db/model"
	"github.com/mindmain/eattura/sdi/fe"

	"github.com/mindmain/eattura/fs"
	"github.com/mindmain/eattura/pec"
)

type Status = model.Status
type Role = model.BillingRole
type Type = model.BillingType

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

type Issuer struct {
	// Title is optional field, used for the title of the issuer. ex. Dott. Ing. Prof.
	Title        string `json:"title"`
	Name         string `json:"name"`
	Surname      string `json:"surname"`
	Denomination string `json:"denomination"`

	VatCode    *VatCode `json:"vat_code"`
	FiscalCode string   `json:"fiscal_code"`
	Address    *Address `json:"address"`

	Phone string `json:"phone"`
	Email string `json:"email"`

	// (Economic Operator Registration and Identification) is a unique code assigned to
	// economic operators and other persons registered in the European Union (EU),
	// ignore this field if not present, not required for normal invoice.
	CodEORI string `json:"cod_eori"`

	Rea *Rea `json:"rea"`

	uuidCredential string
}

type Contact struct {
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
	Title   string `json:"title"`
	CodEORI string `json:"cod_eori"`
}

type Customer struct {
	Contact `json:",inline"`
	Pec     string `json:"pec"`
	//(Economic Operator Registration and Identification) is a unique code assigned to economic operators and other persons registered in the European Union (EU) for customs purposes.
}

type Supplier struct {
	Contact `json:",inline"`
	Rea     *Rea `json:"rea,omitempty"`
}

type Natura = fe.Natura

const (
	NatureNoIvaSubject        Natura = fe.N_2
	NatureNoIvaOtherCases     Natura = fe.N_2_2
	NatureExcludedByArticle15 Natura = fe.N_1
	NatureExport              Natura = fe.N_3_1
)

type InvoiceItem struct {
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
	Type TypeDocument `json:"type"`
	// If use issuer static mode on config you can omit the issuer field.
	Issuer          *Issuer   `json:"issuer"`
	Supplier        *Supplier `json:"supplier"`
	Customer        *Customer `json:"customer"`
	DestinationCode string    `json:"destination_code"`

	Date   time.Time `json:"date"`
	Number string    `json:"number"`

	UniqueNumber      string `json:"unique_number"`
	ProgressiveNumber string `json:"progressive_number"`

	Items []*InvoiceItem `json:"items"`
}

func (i *Invoice) Total() float64 {
	var total float64
	for _, item := range i.Items {
		total += item.Total()
	}
	return total
}

type RequestCreateInvoice struct {

	// Save save the invoice on the db. if the db not available, return error.
	// this flag not create Issuer and Contact if not present.
	Save bool `json:"save"`

	// SaveXML save the XML file on the fs. if the fs not available, return error.
	SaveXML bool `json:"save_xml"`

	// Send send the invoice to the PEC service, if not possible return error.
	Sent    bool     `json:"sent"`
	Invoice *Invoice `json:"invoice"`
}

type ResponseCreateInvoice struct {
	SentAt  time.Time      `json:"sent_at,omitempty"`
	Invoice *model.Invoice `json:"invoice"`
}

type FinderInvoice interface {
	WithStatus(status Status) FinderInvoice

	Count(ctx context.Context) (int, error)
	List(ctx context.Context, offset, limit int) ([]*Invoice, error)
}

type Invoicer interface {
	CreateInvoice(ctx context.Context, request *RequestCreateInvoice) (*ResponseCreateInvoice, error)

	ReadInvoice(ctx context.Context, uuid string) (*model.Invoice, error)
	UpdateInvoice(ctx context.Context, invoice *model.Invoice) error
	DeleteInvoice(ctx context.Context, uuid string) error

	InvoiceFinder() FinderInvoice

	Issuers(ctx context.Context) ([]*model.Issuer, error)
	GetIssuer(ctx context.Context, uuid string) (*model.Issuer, error)
	// SendInvoice send the invoice to the PEC service, first check if the invoice is valid
	// - check if the invoice is already with final status: [sent, paid, received]
	// - check if the invoice is ok for internal SDI system.
	// - send the invoice to the PEC service
	SendInvoice(ctx context.Context, uuid string) error
}

type FinderContact interface {
	WithRole(role model.BillingRole) FinderContact
	WithType(t Type) FinderContact
	Count(ctx context.Context) (int, error)
	List(ctx context.Context, offset, limit int) ([]*Customer, error)
}

type Rubric interface {
	CreateContact(ctx context.Context, contact *model.Contact) error
	ReadContact(ctx context.Context, uuid string) (*model.Contact, error)
	UpdateContact(ctx context.Context, contact *model.Contact) error
	DeleteContact(ctx context.Context, uuid string) error

	FinderContact() FinderContact

	SetRole(ctx context.Context, uuid string, role model.BillingRole) error
	SetBillingType(ctx context.Context, uuid string, billingType model.BillingType) error
}

// The Credential info is consider immutable.
// This handler provide correct way to manage password and other secret information.
type HandlerCredential interface {
	CreateCredential(ctx context.Context, credential *model.Credentials) error
	DeleteCredential(ctx context.Context, uuid string) (*model.Credentials, error)
	List(ctx context.Context) ([]*model.Credentials, error)
}

var ErrInconsistentData = errors.New("inconsistent data")

type Eattura interface {
	Invoicer
	Rubric
	HandlerCredential
	GeneratorProgressive
	Loader
}

func New(
	database db.Database,
	pecClient pec.Client,
	invoiceFolder fs.InvoiceFileHandler,
) Eattura {

	var issuerHandler = &issuerHandler{db: database}

	return &service{
		Loader: &defaultLoader{
			db:            database,
			pec:           pecClient,
			store:         invoiceFolder,
			issuerHandler: issuerHandler,
		},
		Invoicer:             nil,
		Rubric:               nil,
		HandlerCredential:    nil,
		GeneratorProgressive: &defaultGeneratorProgressive{},
	}
}

type service struct {
	HandlerCredential
	Invoicer
	Rubric
	Loader
	GeneratorProgressive
}
