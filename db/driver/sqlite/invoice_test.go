//go:build sqlite

package sqlite

import (
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/mindmain/eattura/db/model"
	"github.com/stretchr/testify/assert"
)

func TestSimpleCrudInvoice(t *testing.T) {

	t.Run("test create an invoice and retrive, should be the same", func(t *testing.T) {

		invoice := &model.Invoice{
			UUID: "1",
		}

		err := db.Invoice().Create(context.TODO(), invoice)

		if !assert.NoError(t, err) {
			return
		}

		invoice, err = db.Invoice().Read(context.TODO(), "1")

		if !assert.NoError(t, err) {
			return
		}
		defer db.Invoice().Delete(context.TODO(), "1")

		assert.Equal(t, "1", invoice.UUID)
	})

	t.Run("test update an invoice and retrive, should be the same", func(t *testing.T) {

		invoice := &model.Invoice{
			UUID: "2",
		}

		err := db.Invoice().Create(context.TODO(), invoice)

		if !assert.NoError(t, err) {
			return
		}
		defer db.Invoice().Delete(context.TODO(), "2")
		now := time.Now()
		err = db.Invoice().Update(context.TODO(), "2", &model.Invoice{
			UUID: "2",
			Date: now,
		})

		if !assert.NoError(t, err) {
			return
		}

		invoice, err = db.Invoice().Read(context.TODO(), "2")

		if !assert.NoError(t, err) {
			return
		}

		defer db.Invoice().Delete(context.TODO(), "2")

		assert.Equal(t, "2", invoice.UUID)
		assert.Equal(t, now.Format("2006-01-02"), invoice.Date.Format("2006-01-02"))
	})

	t.Run("test find invoices", func(t *testing.T) {

		status := make(map[int]model.Status)

		status[0] = model.StatusDraft
		status[1] = model.StatusSent
		status[2] = model.StatusPaid
		status[3] = model.StatusReceived
		status[4] = model.StatusDraft
		status[5] = model.StatusSent
		status[6] = model.StatusPaid
		status[7] = model.StatusPartialPaid
		status[8] = model.StatusPaid
		status[9] = model.StatusPaid

		for i := 0; i < 10; i++ {
			invoice := &model.Invoice{
				UUID:              fmt.Sprintf("test-%d", i),
				ProgressiveNumber: fmt.Sprintf("progressive-test-%d", i),
				UniqueProgressive: fmt.Sprintf("unique-%d", i),
				Status:            status[i],
			}

			err := db.Invoice().Create(context.TODO(), invoice)

			if !assert.NoError(t, err) {
				return
			}

			defer db.Invoice().Delete(context.TODO(), fmt.Sprintf("%d", i))
		}

		t.Run("simple find", func(t *testing.T) {
			invoices, err := db.Invoice().Find(context.TODO(), 0, 10, &model.RequestSearchInvoice{})

			if !assert.NoError(t, err) {
				return
			}

			assert.Len(t, invoices, 10)
		})

		t.Run("find with limit", func(t *testing.T) {
			invoices, err := db.Invoice().Find(context.TODO(), 0, 5, &model.RequestSearchInvoice{})

			if !assert.NoError(t, err) {
				return
			}

			assert.Len(t, invoices, 5)
		})

		t.Run("find with skip", func(t *testing.T) {
			invoices, err := db.Invoice().Find(context.TODO(), 9, 5, &model.RequestSearchInvoice{})

			if !assert.NoError(t, err) {
				return
			}

			assert.Len(t, invoices, 1)
		})

		t.Run("find with filter status", func(t *testing.T) {
			invoices, err := db.Invoice().Find(context.TODO(), 0, 10, &model.RequestSearchInvoice{
				Status: []model.Status{model.StatusDraft},
			})

			if !assert.NoError(t, err) {
				return
			}

			assert.Len(t, invoices, 2)
		})
	})

}

func TestFindInvoiceWithPreload(t *testing.T) {

	t.Run("test find with items and contacts and issuer", func(t *testing.T) {

		contact := &model.Contact{
			UUID:    "1",
			Name:    "Mario",
			Surname: "Rossi",
		}

		err := db.Contact().Create(context.TODO(), contact)

		if !assert.NoError(t, err) {
			return
		}

		defer db.Contact().Delete(context.TODO(), "1")

		issuer := &model.Issuer{
			UUID:             "issuer1",
			ContactReference: "issuer1",
			Contact: &model.Contact{
				UUID:         "issuer1",
				Denomination: "My Company",
			},
		}

		err = db.Issuer().Create(context.TODO(), issuer)

		if !assert.NoError(t, err) {
			return
		}

		defer db.Issuer().Delete(context.TODO(), "1")

		invoice := &model.Invoice{
			UUID:              "t1",
			CustomerReference: "1",
			SupplierReference: "1",
			IssuerReference:   "issuer1",
			ProgressiveNumber: "1",
			UniqueProgressive: "1",
		}

		err = db.Invoice().Create(context.TODO(), invoice)

		if !assert.NoError(t, err) {
			return
		}

		for i := 0; i < 10; i++ {
			item := &model.InvoiceItem{
				UUID:        fmt.Sprintf("%d", i),
				InvoiceUUID: "t1",
				Description: fmt.Sprintf("item %d", i),
				UnitAmount:  10.0,
			}

			err := db.InvoiceItem().Create(context.TODO(), item)

			if !assert.NoError(t, err) {
				return
			}
		}

		defer db.Invoice().Delete(context.TODO(), "1")

		invoice, err = db.Invoice().Read(context.TODO(), "t1")

		if !assert.NoError(t, err) {
			return
		}

		assert.Equal(t, "t1", invoice.UUID)
		if assert.NotNil(t, invoice.Customer, "Customer is nil value: %s", invoice.CustomerReference) {
			assert.Equal(t, "1", invoice.Customer.UUID)
			assert.Equal(t, "Mario", invoice.Customer.Name)
			assert.Equal(t, "Rossi", invoice.Customer.Surname)
		}

		if assert.NotNil(t, invoice.Issuer, "issuer is nil value: %s", invoice.IssuerReference) {
			assert.Equal(t, "issuer1", invoice.Issuer.UUID)
			assert.Equal(t, "My Company", invoice.Issuer.Contact.Denomination)
		}

		assert.Len(t, invoice.Items, 10)
	})
}
