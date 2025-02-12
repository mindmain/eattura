//go:build sqlite

package sqlite

import (
	"context"

	"github.com/mindmain/eattura/db/model"
	"xorm.io/xorm"
)

type issuerRepository struct {
	db *xorm.Engine
}

func (i *issuerRepository) Create(ctx context.Context, m *model.Issuer) error {
	return nil
}

func (i *issuerRepository) Read(ctx context.Context, uuid string) (*model.Issuer, error) {
	return &model.Issuer{}, nil
}

func (i *issuerRepository) Update(ctx context.Context, uuid string, m *model.Issuer) error {
	return nil
}

func (i *issuerRepository) Delete(ctx context.Context, uuid string) (*model.Issuer, error) {
	return &model.Issuer{}, nil
}
