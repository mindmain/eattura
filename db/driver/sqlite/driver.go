//go:build sqlite
// +build sqlite

package sqlite

import (
	"fmt"

	_ "github.com/mattn/go-sqlite3" // Importazione anonima per registrare il driver
	"github.com/mindmain/eattura/db/driver"
	"github.com/mindmain/eattura/db/model"
	gSqlite "gorm.io/driver/sqlite"
	"gorm.io/gorm"
)

func init() {
	driver.Register("sqlite3", New)
}

type sqliteDb struct {
	db *gorm.DB
}

func New(config *driver.DatabaseConfig) (driver.Database, error) {

	db, err := gorm.Open(gSqlite.Open(config.Uri), &gorm.Config{})
	if err != nil {
		return nil, err
	}

	return &sqliteDb{
		db: db,
	}, nil
}

func (s *sqliteDb) Init() error {

	if err := s.db.AutoMigrate(&model.Invoice{}); err != nil {
		return fmt.Errorf("failed to migrate model Invoice: %w", err)
	}
	if err := s.db.AutoMigrate(&model.Contact{}); err != nil {
		return fmt.Errorf("failed to migrate model Contact: %w", err)
	}
	if err := s.db.AutoMigrate(&model.Credentials{}); err != nil {
		return fmt.Errorf("failed to migrate model Credentials: %w", err)
	}
	if err := s.db.AutoMigrate(&model.InvoiceItem{}); err != nil {
		return fmt.Errorf("failed to migrate model InvoiceItem: %w", err)
	}
	if err := s.db.AutoMigrate(&model.Issuer{}); err != nil {
		return fmt.Errorf("failed to migrate model Issuer: %w", err)
	}
	if err := s.db.AutoMigrate(&model.Notification{}); err != nil {
		return fmt.Errorf("failed to migrate model Notification: %w", err)
	}

	return nil
}

func (s *sqliteDb) Ping() error {
	return nil
}

func (s *sqliteDb) Close() error {

	return nil

}

func (s *sqliteDb) Invoice() driver.InvoiceRepository {
	return &invoiceRepository{
		db: s.db,
	}
}

func (s *sqliteDb) Contact() driver.ContactRepository {
	return &contactRepository{
		db: s.db,
	}
}

func (s *sqliteDb) Credential() driver.CredentialRepository {
	return &credentialRepository{
		db: s.db,
	}
}

func (s *sqliteDb) InvoiceItem() driver.InvoiceItemRepository {
	return &invoiceItemRepository{
		db: s.db,
	}
}

func (s *sqliteDb) Issuer() driver.IssuerRepository {
	return &issuerRepository{
		db: s.db,
	}
}

func (s *sqliteDb) Notification() driver.NotificationRepository {
	return &invoiceNotificationRepository{
		db: s.db,
	}
}
