//go:build sqlite

package sqlite

import (
	"context"

	"github.com/mindmain/eattura/db/model"
	"gorm.io/gorm"
)

type invoiceItemRepository struct {
	db *gorm.DB
}

func (i *invoiceItemRepository) Create(ctx context.Context, m *model.InvoiceItem) error {

	result := i.db.Create(m)

	if result.Error != nil {
		return result.Error
	}

	return nil

}

func (i *invoiceItemRepository) Read(ctx context.Context, uuid string) (*model.InvoiceItem, error) {

	invoiceItem := &model.InvoiceItem{}
	result := i.db.First(invoiceItem, uuid)

	if result.Error != nil {
		return nil, result.Error
	}

	return invoiceItem, nil

}

func (i *invoiceItemRepository) Update(ctx context.Context, uuid string, m *model.InvoiceItem) error {

	result := i.db.Where("uuid = ?", uuid).Updates(m)

	return result.Error

}

func (i *invoiceItemRepository) Delete(ctx context.Context, uuid string) error {

	result := i.db.Delete(&model.InvoiceItem{}, uuid)

	if result.Error != nil {
		return result.Error
	}

	return nil
}

func (s *invoiceItemRepository) Find(ctx context.Context, skip, limit uint, req *model.RequestSearchInvoiceItem) ([]*model.InvoiceItem, error) {

	session := s.session(req)

	if skip > 0 || limit > 0 {
		session = session.Limit(int(limit)).Offset(int(skip))
	}

	var invoiceItems []*model.InvoiceItem

	result := session.Find(&invoiceItems)

	if result.Error != nil {
		return nil, result.Error
	}

	return invoiceItems, nil

}

func (s *invoiceItemRepository) Count(ctx context.Context, req *model.RequestSearchInvoiceItem) (int, error) {

	session := s.session(req)

	var count int64

	result := session.Count(&count)

	if result.Error != nil {
		return 0, result.Error
	}

	return int(count), nil

}

func (s *invoiceItemRepository) session(req *model.RequestSearchInvoiceItem) *gorm.DB {
	session := s.db.Session(&gorm.Session{
		NewDB: true,
	})

	if req != nil {
		req = &model.RequestSearchInvoiceItem{}
	}

	if len(req.InvoiceUUID) > 0 {
		session = session.Where("invoice_uuid IN (?)", req.InvoiceUUID)
	}

	return session
}
