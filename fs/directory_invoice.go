package fs

import (
	"context"
	"path"
	"time"

	"github.com/mindmain/eattura/fs/driver"
)

type directoryInvoice struct {
	driver driver.FileDriver
	year   int
	month  time.Month
	folder string
}

func (d *directoryInvoice) Year() int {
	return d.year
}

func (d *directoryInvoice) Month() time.Month {
	return d.month
}

func (d *directoryInvoice) Path() string {
	return d.folder
}

func (d *directoryInvoice) Invoices(ctx context.Context) ([]FileReference, error) {

	files, err := d.driver.Ls(d.folder)

	if err != nil {
		return nil, err
	}

	var refs []FileReference

	for _, file := range files {
		refs = append(refs, &invoiceReference{
			filepath: path.Join(d.folder, file),
			driver:   d.driver,
		})
	}

	return refs, nil
}
