package eattura

import (
	"context"

	"github.com/google/uuid"
	"github.com/mindmain/eattura/internal/db"
	"github.com/mindmain/eattura/internal/db/model"
	"github.com/mindmain/eattura/internal/secure"
)

func determinateCredentialUUID(cred *CredentialSetting) string {
	return uuid.NewSHA1(uuid.MustParse(NamespaceEattura), []byte(cred.Username)).String()
}

// The Credential info is consider immutable.
// This handler provide correct way to manage password and other secret information.
type handlerCredential interface {
	GetCredential(ctx context.Context, uuid string) (*Credential, error)
	CreateCredential(ctx context.Context, credential *CredentialSetting) (*Credential, error)
	DeleteCredential(ctx context.Context, uuid string) (*Credential, error)
	List(ctx context.Context) ([]*Credential, error)
}

type handlerCredImpl struct {
	db     db.Database
	secure secure.Storage
}

func (c *handlerCredImpl) GetCredential(ctx context.Context, uuid string) (*Credential, error) {
	ref, err := c.db.Credential().Read(ctx, uuid)

	if err != nil {
		return nil, err
	}

	creds, err := c.secure.Get(ref.UUID)

	if err != nil {
		return nil, err
	}

	return &Credential{
		ID:      ref.UUID,
		Setting: creds,
	}, nil
}

func (c *handlerCredImpl) CreateCredential(ctx context.Context, credential *CredentialSetting) (*Credential, error) {

	uuid := determinateCredentialUUID(credential)

	err := c.secure.Create(uuid, credential)

	if err != nil {
		return nil, err
	}

	err = c.db.Credential().Create(ctx, &model.Credentials{
		UUID: uuid,
		Type: model.TypePec,
	})

	if err != nil {
		return nil, err
	}

	return &Credential{
		ID:      uuid,
		Setting: credential,
	}, nil
}

func (c *handlerCredImpl) DeleteCredential(ctx context.Context, uuid string) (*Credential, error) {

	creds, err := c.GetCredential(ctx, uuid)

	if err != nil {
		return nil, err
	}

	err = c.secure.Delete(uuid)

	if err != nil {
		return nil, err
	}

	err = c.db.Credential().Delete(ctx, uuid)

	if err != nil {
		return nil, err
	}

	return creds, nil
}

func (c *handlerCredImpl) List(ctx context.Context) ([]*Credential, error) {

	refs, err := c.db.Credential().FindAll(ctx)

	if err != nil {
		return nil, err
	}

	creds := make([]*Credential, 0, len(refs))

	for _, ref := range refs {

		cred, err := c.GetCredential(ctx, ref.UUID)

		if err != nil {
			return nil, err
		}

		creds = append(creds, cred)

	}

	return creds, nil
}
