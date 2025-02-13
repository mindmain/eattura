//go:build sqlite

package sqlite

import (
	"context"

	"github.com/mindmain/eattura/db/model"
	"gorm.io/gorm"
)

type issuerRepository struct {
	db *gorm.DB
}

func (i *issuerRepository) Create(ctx context.Context, m *model.Issuer) error {

	result := i.db.Create(m)

	if result.Error != nil {
		return result.Error
	}

	return nil

}

func (i *issuerRepository) Read(ctx context.Context, uuid string) (*model.Issuer, error) {

	issuer := &model.Issuer{}
	result := i.db.First(issuer, uuid)

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
