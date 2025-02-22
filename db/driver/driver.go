package driver

import (
	"context"
	"fmt"

	"github.com/mindmain/eattura/db/model"
)

type CreateModel[M any] interface {
	Create(ctx context.Context, m *M) error
}

type ReadModel[M any] interface {
	Read(ctx context.Context, uuid string) (*M, error)
}

type UpdateModel[M any] interface {
	Update(ctx context.Context, uuid string, m *M) error
}

type DeleteModel interface {
	Delete(ctx context.Context, uuid string) error
}

type CrudRepository[M any] interface {
	CreateModel[M]
	ReadModel[M]
	UpdateModel[M]
	DeleteModel
}

type Repository[R any, M any] interface {
	CrudRepository[M]
	Find(ctx context.Context, skip, limit uint, req *R) ([]*M, error)
	Count(ctx context.Context, req *R) (int, error)
}

type InvoiceRepository interface {
	Repository[model.RequestSearchInvoice, model.Invoice]
}

type InvoiceItemRepository interface {
	Repository[model.RequestSearchInvoiceItem, model.InvoiceItem]
}

type NotificationRepository interface {
	CreateModel[model.Notification]
	ReadModel[model.Notification]
}

type IssuerRepository interface {
	CrudRepository[model.Issuer]
	FindAll(ctx context.Context) ([]*model.Issuer, error)
}

type ContactRepository interface {
	Repository[model.RequestSearchContact, model.Contact]
}

type CredentialRepository interface {
	CrudRepository[model.Credentials]

	FindAll(ctx context.Context) ([]*model.Credentials, error)
}

type Database interface {
	Init() error
	Ping() error
	Invoice() InvoiceRepository
	Contact() ContactRepository
	Credential() CredentialRepository
	InvoiceItem() InvoiceItemRepository

	Issuer() IssuerRepository
	Notification() NotificationRepository

	Close() error
}

var drivers = make(map[string]func(*DatabaseConfig) (Database, error))

type DatabaseConfig struct {
	Uri   string
	Debug bool
}

func Register(name string, driver func(*DatabaseConfig) (Database, error)) {
	drivers[name] = driver
}

func GetDriver(name string) (func(*DatabaseConfig) (Database, error), error) {
	if f, ok := drivers[name]; ok {
		return f, nil
	}

	return nil, fmt.Errorf("database driver not found")
}
