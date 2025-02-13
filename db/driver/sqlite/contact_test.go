//go:build sqlite

package sqlite

import (
	"context"
	"fmt"
	"testing"

	"github.com/mindmain/eattura/db/model"
	"github.com/stretchr/testify/assert"
)

func TestContactSimpleCrud(t *testing.T) {

	t.Run("test create an contact and retrive, should be the same", func(t *testing.T) {
		err := db.Contact().Create(context.TODO(), &model.Contact{
			UUID:  "1",
			Name:  "John Doe",
			Email: "example@email.com",
			Phone: "1234567890",
		})

		if !assert.NoError(t, err) {
			return
		}

		contact, err := db.Contact().Read(context.TODO(), "1")

		if !assert.NoError(t, err) {
			return
		}

		defer db.Contact().Delete(context.TODO(), "1")

		assert.Equal(t, "1", contact.UUID)
		assert.Equal(t, "John Doe", contact.Name)
		assert.Equal(t, "example@email.com", contact.Email)
	})

	t.Run("test update an contact and retrive, should be the same", func(t *testing.T) {
		err := db.Contact().Create(context.TODO(), &model.Contact{
			UUID:  "2",
			Name:  "John Doe",
			Email: "eee@email.com",
			Phone: "1234567890",
		})

		if !assert.NoError(t, err) {
			return
		}

		err = db.Contact().Update(context.TODO(), "2", &model.Contact{
			UUID:  "2",
			Name:  "Jane Doe",
			Email: "1@e.com",
			Phone: "1234567890",
		})

		if !assert.NoError(t, err) {
			return
		}

		contact, err := db.Contact().Read(context.TODO(), "2")

		if !assert.NoError(t, err) {
			return
		}

		defer db.Contact().Delete(context.TODO(), "2")

		assert.Equal(t, "2", contact.UUID)
		assert.Equal(t, "Jane Doe", contact.Name)
		assert.Equal(t, "1@e.com", contact.Email)
	})

	t.Run("test find contacts", func(t *testing.T) {

		for i := 0; i < 10; i++ {

			var role model.BillingRole = model.BillingRoleCustomer
			if i%2 == 0 {
				role = model.BillingRoleSupplier
			}

			var bill_type model.BillingType = model.BillingTypePerson

			if i%2 == 0 {
				bill_type = model.BillingTypeCompany
			}

			err := db.Contact().Create(context.TODO(), &model.Contact{
				UUID:        fmt.Sprintf("%d", i),
				Name:        fmt.Sprintf("Mario Rossi %d", i),
				Email:       fmt.Sprintf("email%d@test.com", i),
				Phone:       "1234567890",
				Role:        role,
				BillingType: bill_type,
			})

			if !assert.NoError(t, err) {
				return
			}

			defer db.Contact().Delete(context.TODO(), fmt.Sprintf("%d", i))
		}

		t.Run(fmt.Sprintf("test find contacts with role"), func(t *testing.T) {

			contacts, err := db.Contact().Find(context.TODO(), 0, 10, &model.RequestSearchContact{
				Role: []model.BillingRole{model.BillingRoleCustomer},
			})

			if !assert.NoError(t, err) {
				return
			}

			assert.Len(t, contacts, 5)
		})

		t.Run(fmt.Sprintf("test find contacts with billing type"), func(t *testing.T) {

			contacts, err := db.Contact().Find(context.TODO(), 0, 10, &model.RequestSearchContact{
				BillingType: []model.BillingType{model.BillingTypeCompany},
			})

			if !assert.NoError(t, err) {
				return
			}

			assert.Len(t, contacts, 5)
		})

		t.Run(fmt.Sprintf("test find contacts with text"), func(t *testing.T) {

			contacts, err := db.Contact().Find(context.TODO(), 0, 10, &model.RequestSearchContact{
				Text: "Mario Rossi 1",
			})

			if !assert.NoError(t, err) {
				return
			}

			assert.Len(t, contacts, 1)
		})

	})
}
