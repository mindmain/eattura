//go:build sqlite
// +build sqlite

package sqlite

import (
	_ "github.com/mattn/go-sqlite3" // Importazione anonima per registrare il driver
	"github.com/mindmain/eattura/db/driver"
	"github.com/mindmain/eattura/db/model"
	"xorm.io/xorm"
)

func init() {
	driver.Register("sqlite3", New)
}

type sqliteDb struct {
	db *xorm.Engine
}

func New(config *driver.DatabaseConfig) (driver.Database, error) {

	engine, err := xorm.NewEngine("sqlite3", config.Uri)

	if err != nil {
		return nil, err
	}

	return &sqliteDb{
		db: engine,
	}, nil
}

func (s *sqliteDb) Init() error {

	if err := s.db.Sync(new(model.Contact)); err != nil {
		return err
	}
	if err := s.db.Sync(new(model.Credentials)); err != nil {
		return err
	}
	if err := s.db.Sync(new(model.Invoice)); err != nil {
		return err
	}
	if err := s.db.Sync(new(model.InvoiceItem)); err != nil {
		return err
	}
	if err := s.db.Sync(new(model.Issuer)); err != nil {
		return err
	}
	if err := s.db.Sync(new(model.Notification)); err != nil {
		return err
	}

	return nil
}

func (s *sqliteDb) Ping() error {
	return nil
}

func (s *sqliteDb) Close() error {

	return nil

}

func (s *sqliteDb) Invoice() driver.Repository[model.RequestSearchInvoice, model.Invoice] {
	return &invoiceRepository{
		db: s.db,
	}
}

func (s *sqliteDb) Contact() driver.Repository[model.RequestSearchContact, model.Contact] {
	return &contactRepository{
		db: s.db,
	}
}

func (s *sqliteDb) Credential() driver.CrudRepository[model.Credentials] {
	return &credentialRepository{
		db: s.db,
	}
}

func (s *sqliteDb) InvoiceItem() driver.Repository[model.RequestSearchInvoiceItem, model.InvoiceItem] {
	return &invoiceItemRepository{
		db: s.db,
	}
}

func (s *sqliteDb) Issuer() driver.CrudRepository[model.Issuer] {
	return &issuerRepository{
		db: s.db,
	}
}

func (s *sqliteDb) Notification() driver.CrudRepository[model.Notification] {
	return &invoiceNotificationRepository{
		db: s.db,
	}
}
