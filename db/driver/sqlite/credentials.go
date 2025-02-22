//go:build sqlite

package sqlite

import (
	"context"

	"github.com/mindmain/eattura/db/model"
	"gorm.io/gorm"
)

type credentialRepository struct {
	db *gorm.DB
}

func (i *credentialRepository) Create(ctx context.Context, m *model.Credentials) error {

	result := i.db.Create(m)

	if result.Error != nil {
		return result.Error
	}

	return nil

}

func (i *credentialRepository) Read(ctx context.Context, uuid string) (*model.Credentials, error) {

	var cred model.Credentials

	result := i.db.Where("uuid = ?", uuid).First(&cred)

	if result.Error != nil {
		return nil, result.Error
	}

	return &cred, nil

}

func (i *credentialRepository) Update(ctx context.Context, uuid string, m *model.Credentials) error {

	if m.UUID != uuid {
		return gorm.ErrRecordNotFound
	}

	result := i.db.Save(m)

	if result.Error != nil {
		return result.Error
	}

	return nil

}

func (i *credentialRepository) Delete(ctx context.Context, uuid string) error {

	result := i.db.Where("uuid = ?", uuid).Delete(&model.Credentials{})

	if result.Error != nil {
		return result.Error
	}

	return nil

}

func (i *credentialRepository) FindAll(ctx context.Context) ([]*model.Credentials, error) {

	var creds []*model.Credentials

	result := i.db.Find(&creds)

	if result.Error != nil {
		return nil, result.Error
	}

	return creds, nil

}
