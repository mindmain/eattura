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

	return nil
}

func (i *credentialRepository) Read(ctx context.Context, uuid string) (*model.Credentials, error) {
	return &model.Credentials{}, nil
}

func (i *credentialRepository) Update(ctx context.Context, uuid string, m *model.Credentials) error {
	return nil
}

func (i *credentialRepository) Delete(ctx context.Context, uuid string) error {
	return nil
}
