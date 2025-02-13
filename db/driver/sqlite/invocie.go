//go:build sqlite

package sqlite

import (
	"context"

	"github.com/mindmain/eattura/db/model"
	"gorm.io/gorm"
)

type invoiceRepository struct {
	db *gorm.DB
}

func (i *invoiceRepository) Create(ctx context.Context, m *model.Invoice) error {

	result := i.db.Create(m)

	if result.Error != nil {
		return result.Error
	}

	return nil

}

func (i *invoiceRepository) Read(ctx context.Context, uuid string) (*model.Invoice, error) {

	invoice := &model.Invoice{}
	result := i.db.Preload("Contact").Preload("Items").Preload("Notifications").Preload("Issuer").First(invoice, uuid)

	if result.Error != nil {
		return nil, result.Error
	}

	return invoice, nil

}

func (i *invoiceRepository) Update(ctx context.Context, uuid string, m *model.Invoice) error {

	result := i.db.Where("uuid = ?", uuid).Updates(m)

	return result.Error

}

func (i *invoiceRepository) Delete(ctx context.Context, uuid string) error {

	result := i.db.Delete(&model.Invoice{}, uuid)

	if result.Error != nil {
		return result.Error
	}

	return nil

}

func (s *invoiceRepository) Find(ctx context.Context, skip, limit uint, req *model.RequestSearchInvoice) ([]*model.Invoice, error) {

	invoices := make([]*model.Invoice, 0)
	session := s.session(req).Preload("Contact").Preload("Items").Preload("Notifications").Preload("Issuer")

	if skip > 0 || limit > 0 {
		session = session.Limit(int(limit)).Offset(int(skip))
	}

	result := session.Find(&invoices)

	if result.Error != nil {
		return nil, result.Error
	}

	return invoices, nil

}

func (s *invoiceRepository) Count(ctx context.Context, req *model.RequestSearchInvoice) (int, error) {

	var count int64
	err := s.session(req).Count(&count)

	if err.Error != nil {
		return 0, err.Error
	}

	return int(count), nil

}

func (s *invoiceRepository) session(req *model.RequestSearchInvoice) *gorm.DB {
	session := s.db.Session(&gorm.Session{
		NewDB: true,
	}).Model(&model.Invoice{})

	if req == nil {
		req = &model.RequestSearchInvoice{}
	}

	if len(req.ContactUUID) > 0 {
		session = session.Where("contact_uuid IN (?)", req.ContactUUID)
	}

	if len(req.IssuerUUID) > 0 {
		session = session.Where("issuer_uuid IN (?)", req.IssuerUUID)
	}

	if len(req.Status) > 0 {

		session = session.Where("status IN (?)", req.Status)
	}

	if req.StartDate != nil {
		session = session.Where("invoice_date >= ?", req.StartDate)
	}

	if req.EndDate != nil {
		session = session.Where("invoice_date <= ?", req.EndDate)
	}

	return session
}
