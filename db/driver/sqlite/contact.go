//go:build sqlite

package sqlite

import (
	"context"

	"github.com/mindmain/eattura/db/model"
	"gorm.io/gorm"
)

type contactRepository struct {
	db *gorm.DB
}

func (i *contactRepository) Create(ctx context.Context, m *model.Contact) error {

	result := i.db.Create(m)

	return result.Error

}

func (i *contactRepository) Read(ctx context.Context, uuid string) (*model.Contact, error) {

	m := &model.Contact{}

	result := i.db.First(m, uuid)

	if result.Error != nil {
		return nil, result.Error
	}

	return m, nil

}

func (i *contactRepository) Update(ctx context.Context, uuid string, m *model.Contact) error {

	result := i.db.Where("uuid = ?", uuid).Updates(m)

	return result.Error
}

func (i *contactRepository) Delete(ctx context.Context, uuid string) error {

	m := &model.Contact{}
	result := i.db.Delete(m, uuid)
	return result.Error

}

func (s *contactRepository) Find(ctx context.Context, skip, limit uint, req *model.RequestSearchContact) ([]*model.Contact, error) {

	session := s.session(req)

	if skip > 0 || limit > 0 {
		session = session.Limit(int(limit)).Offset(int(skip))
	}

	var contacts []*model.Contact

	result := session.Find(&contacts)

	if result.Error != nil {
		return nil, result.Error
	}

	return contacts, nil
}

func (s *contactRepository) Count(ctx context.Context, req *model.RequestSearchContact) (int, error) {

	session := s.session(req)
	var count int64
	err := session.Count(&count)

	if err.Error != nil {
		return 0, err.Error
	}

	return int(count), nil
}

func (s *contactRepository) session(req *model.RequestSearchContact) *gorm.DB {
	session := s.db.Session(&gorm.Session{
		NewDB: true,
	})

	if req == nil {
		req = &model.RequestSearchContact{}
	}
	if len(req.Role) > 0 {
		session = session.Where("role IN (?)", req.Role)
	}

	if len(req.BillingType) > 0 {
		session = session.Where("type IN (?)", req.BillingType)
	}

	if req.Text != "" {
		session = session.Where("name LIKE ? OR email LIKE ? OR phone LIKE ? OR street LIKE ?", "%"+req.Text+"%", "%"+req.Text+"%", "%"+req.Text+"%", "%"+req.Text+"%")
	}

	return session
}
