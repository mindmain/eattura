//go:build sqlite

package sqlite

import (
	"context"

	"github.com/mindmain/eattura/db/model"
	"xorm.io/xorm"
)

type credentialRepository struct {
	db *xorm.Engine
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

func (i *credentialRepository) Delete(ctx context.Context, uuid string) (*model.Credentials, error) {
	return &model.Credentials{}, nil
}
