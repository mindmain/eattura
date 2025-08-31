package db

import (
	"fmt"
	"log"
	"os"

	"github.com/mindmain/eattura/internal/core"
	"github.com/mindmain/eattura/internal/db/driver"
)

type Type = core.DBType

const (
	TypeSqlite Type = core.DBTypeSqlite
	TypeNone   Type = core.DBTypeNone
)

type Database = driver.Database

func New() (driver.Database, error) {

	tt := core.GetDatabaseType()

	switch tt {
	case TypeSqlite:

		pathDb := core.Get("database.uri")
		caller, err := driver.GetDriver("sqlite3")

		if err != nil {
			return nil, err
		}

		database, err := caller(&driver.DatabaseConfig{
			Uri: pathDb,
		})

		if err != nil {
			return nil, err
		}

		return database, nil
	case TypeNone:
		return &noneImpl{}, nil
	}

	return nil, fmt.Errorf("database type not supported")
}

type noneImpl struct{}

func (n *noneImpl) Init() error {
	return nil
}

func (n *noneImpl) Ping() error {
	return nil
}

func (n *noneImpl) Close() error {
	return nil
}

func (n *noneImpl) errMsg() {
	log.Fatal("database is none type on config")
	os.Exit(1)
}

func (n *noneImpl) Invoice() driver.InvoiceRepository {
	n.errMsg()
	return nil
}
func (n *noneImpl) Contact() driver.ContactRepository {
	n.errMsg()
	return nil
}
func (n *noneImpl) Credential() driver.CredentialRepository {
	n.errMsg()
	return nil
}
func (n *noneImpl) InvoiceItem() driver.InvoiceItemRepository {
	n.errMsg()
	return nil
}
func (n *noneImpl) Issuer() driver.IssuerRepository {
	n.errMsg()
	return nil
}
func (n *noneImpl) Notification() driver.NotificationRepository {
	n.errMsg()
	return nil
}
