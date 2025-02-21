//go:build sqlite

package sqlite

import (
	"context"
	"fmt"

	"github.com/mindmain/eattura/db/model"
	"gorm.io/gorm"
)

type issuerRepository struct {
	db *gorm.DB
}

func (i *issuerRepository) Create(ctx context.Context, m *model.Issuer) error {

	if m.Contact.UUID != m.UUID {
		return fmt.Errorf("uuid issuer must be same contact uuid")
	}

	result := i.db.Create(m)

	if result.Error != nil {
		return result.Error
	}

	return nil

}

func (i *issuerRepository) Read(ctx context.Context, uuid string) (*model.Issuer, error) {

	issuer := &model.Issuer{}
	result := i.db.Preload("Contact").Where("uuid=(?)", uuid).First(issuer)

	if result.Error != nil {
		return nil, result.Error
	}

	return issuer, nil

}

func (i *issuerRepository) Update(ctx context.Context, uuid string, m *model.Issuer) error {

	result := i.db.Where("uuid = ?", uuid).Updates(m)

	return result.Error
}

func (i *issuerRepository) Delete(ctx context.Context, uuid string) error {

	result := i.db.Delete(&model.Issuer{}, uuid)

	if result.Error != nil {
		return result.Error
	}

	return nil
}

func (i *issuerRepository) FindAll(ctx context.Context) ([]*model.Issuer, error) {

	var issuers []*model.Issuer
	result := i.db.Preload("Contact").Find(&issuers)

	if result.Error != nil {
		return nil, result.Error
	}

	return issuers, nil
}
