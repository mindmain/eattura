//go:build sqlite

package sqlite

import (
	"context"
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

		assert.Equal(t, "2", contact.UUID)
		assert.Equal(t, "Jane Doe", contact.Name)
		assert.Equal(t, "1@e.com", contact.Email)
	})
}
