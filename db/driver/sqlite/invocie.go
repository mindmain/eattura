//go:build sqlite

package sqlite

import (
	"context"

	"github.com/mindmain/eattura/db/model"
	"xorm.io/xorm"
)

type invoiceRepository struct {
	db *xorm.Engine
}

func (i *invoiceRepository) Create(ctx context.Context, m *model.Invoice) error {
	return nil
}

func (i *invoiceRepository) Read(ctx context.Context, uuid string) (*model.Invoice, error) {
	return &model.Invoice{}, nil
}

func (i *invoiceRepository) Update(ctx context.Context, uuid string, m *model.Invoice) error {
	return nil
}

func (i *invoiceRepository) Delete(ctx context.Context, uuid string) (*model.Invoice, error) {
	return &model.Invoice{}, nil
}

func (s *invoiceRepository) Find(ctx context.Context, skip, limit uint, req *model.RequestSearchInvoice) ([]*model.Invoice, error) {
	return []*model.Invoice{}, nil
}

func (s *invoiceRepository) Count(ctx context.Context, req *model.RequestSearchInvoice) (int, error) {
	return 0, nil
}
