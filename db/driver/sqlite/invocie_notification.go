//go:build sqlite

package sqlite

import (
	"context"

	"github.com/mindmain/eattura/db/model"
	"xorm.io/xorm"
)

type invoiceNotificationRepository struct {
	db *xorm.Engine
}

func (i *invoiceNotificationRepository) Create(ctx context.Context, m *model.Notification) error {
	return nil
}

func (i *invoiceNotificationRepository) Read(ctx context.Context, uuid string) (*model.Notification, error) {
	return &model.Notification{}, nil
}

func (i *invoiceNotificationRepository) Update(ctx context.Context, uuid string, m *model.Notification) error {
	return nil
}

func (i *invoiceNotificationRepository) Delete(ctx context.Context, uuid string) (*model.Notification, error) {
	return &model.Notification{}, nil
}
