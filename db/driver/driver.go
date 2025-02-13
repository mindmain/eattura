package driver

import (
	"context"
	"fmt"

	"github.com/mindmain/eattura/db/model"
)

type CrudRepository[M any] interface {
	Create(ctx context.Context, m *M) error
	Read(ctx context.Context, uuid string) (*M, error)
	Update(ctx context.Context, uuid string, m *M) error
	Delete(ctx context.Context, uuid string) error
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

type Database interface {
	Init() error
	Ping() error
	Invoice() InvoiceRepository
	Contact() Repository[model.RequestSearchContact, model.Contact]
	Credential() CrudRepository[model.Credentials]
	InvoiceItem() InvoiceItemRepository

	Issuer() CrudRepository[model.Issuer]
	Notification() CrudRepository[model.Notification]

	Close() error
}

var drivers = make(map[string]func(*DatabaseConfig) (Database, error))

type DatabaseConfig struct {
	Uri string
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
