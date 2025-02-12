//go:build sqlite

package sqlite

import (
	"context"

	"github.com/mindmain/eattura/db/model"
	"xorm.io/xorm"
)

type contactRepository struct {
	db *xorm.Engine
}

func (i *contactRepository) Create(ctx context.Context, m *model.Contact) error {

	_, err := i.db.Insert(m)

	return err

}

func (i *contactRepository) Read(ctx context.Context, uuid string) (*model.Contact, error) {

	m := &model.Contact{}

	_, err := i.db.Where("uuid = ?", uuid).Get(m)

	if err != nil {
		return nil, err
	}

	return m, nil

}

func (i *contactRepository) Update(ctx context.Context, uuid string, m *model.Contact) error {

	_, err := i.db.Where("uuid = ?", uuid).Update(m)

	return err

}

func (i *contactRepository) Delete(ctx context.Context, uuid string) (*model.Contact, error) {

	m := &model.Contact{}

	_, err := i.db.Where("uuid = ?", uuid).Delete(m)

	if err != nil {
		return nil, err
	}

	return m, nil
}

func (s *contactRepository) Find(ctx context.Context, skip, limit uint, req *model.RequestSearchContact) ([]*model.Contact, error) {

	session := s.session(req)

	if skip > 0 || limit > 0 {
		session.Limit(int(limit), int(skip))
	}

	var contacts []*model.Contact

	err := session.Find(&contacts)

	if err != nil {
		return nil, err
	}

	return contacts, nil
}

func (s *contactRepository) Count(ctx context.Context, req *model.RequestSearchContact) (int, error) {

	session := s.session(req)

	count, err := session.Count(&model.Contact{})

	if err != nil {
		return 0, err
	}

	return int(count), nil
}

func (s *contactRepository) session(req *model.RequestSearchContact) *xorm.Session {
	session := s.db.NewSession()

	if req == nil {
		req = &model.RequestSearchContact{}
	}
	if len(req.Role) > 0 {
		session.Where("role IN (?)", req.Role)
	}

	if len(req.BillingType) > 0 {
		session.Where("billing_type IN (?)", req.BillingType)
	}

	if req.Text != "" {
		session.Where("name LIKE ? OR email LIKE ? OR phone LIKE ? OR address LIKE ?", "%"+req.Text+"%", "%"+req.Text+"%", "%"+req.Text+"%", "%"+req.Text+"%")
	}

	return session
}
