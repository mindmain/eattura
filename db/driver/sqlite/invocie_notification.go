//go:build sqlite

package sqlite

import (
	"context"

	"github.com/mindmain/eattura/db/model"
	"gorm.io/gorm"
)

type invoiceNotificationRepository struct {
	db *gorm.DB
}

func (i *invoiceNotificationRepository) Create(ctx context.Context, m *model.Notification) error {

	result := i.db.Create(m)

	if result.Error != nil {
		return result.Error
	}

	return nil

}

func (i *invoiceNotificationRepository) Read(ctx context.Context, uuid string) (*model.Notification, error) {

	invoiceNotification := &model.Notification{}
	result := i.db.First(invoiceNotification, uuid)

	if result.Error != nil {
		return nil, result.Error
	}

	return invoiceNotification, nil
}
