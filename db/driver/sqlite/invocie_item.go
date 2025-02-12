//go:build sqlite

package sqlite

import (
	"context"

	"github.com/mindmain/eattura/db/model"
	"xorm.io/xorm"
)

type invoiceItemRepository struct {
	db *xorm.Engine
}

func (i *invoiceItemRepository) Create(ctx context.Context, m *model.InvoiceItem) error {
	return nil
}

func (i *invoiceItemRepository) Read(ctx context.Context, uuid string) (*model.InvoiceItem, error) {
	return &model.InvoiceItem{}, nil
}

func (i *invoiceItemRepository) Update(ctx context.Context, uuid string, m *model.InvoiceItem) error {
	return nil
}

func (i *invoiceItemRepository) Delete(ctx context.Context, uuid string) (*model.InvoiceItem, error) {
	return &model.InvoiceItem{}, nil
}

func (s *invoiceItemRepository) Find(ctx context.Context, skip, limit uint, req *model.RequestSearchInvoiceItem) ([]*model.InvoiceItem, error) {
	return []*model.InvoiceItem{}, nil
}

func (s *invoiceItemRepository) Count(ctx context.Context, req *model.RequestSearchInvoiceItem) (int, error) {
	return 0, nil
}
