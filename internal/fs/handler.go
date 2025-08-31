package fs

import (
	"context"
	"encoding/xml"
	"fmt"
	"log"
	"path"
	"strconv"
	"time"

	"github.com/mindmain/eattura/internal/sdi"
	"github.com/mindmain/eattura/internal/sdi/fe"
)

func (h *handler) ReadInvoice(ctx context.Context, year, month int, filename string) (*fe.FatturaElettronica, error) {

	bb, err := h.driver.OpenFileRead(path.Join(folderInvoices, fmt.Sprintf("%d", year), fmt.Sprintf("%d", month), filename))

	if err != nil {
		return nil, err
	}

	fat, err := sdi.Read(filename, bb)

	if err != nil {
		return nil, err
	}

	return fat, nil

}

func (h *handler) SaveInvoice(ctx context.Context, filename string, invoice *fe.FatturaElettronica) error {

	date := invoice.Date()

	year := date.Year()
	month := date.Month()

	dirPath := path.Join(folderInvoices, fmt.Sprintf("%d", year), fmt.Sprintf("%d", month))

	if err := h.driver.MkdirAll(dirPath); err != nil {
		return err
	}

	bb, err := xml.Marshal(invoice)

	if err != nil {
		return err
	}
	err = h.driver.WriteFile(path.Join(dirPath, filename), bb)
	return err
}
func (h *handler) List(ctx context.Context) ([]InvoiceDirectoryReference, error) {

	yearsDir, err := h.driver.Ls(folderInvoices)

	if err != nil {
		return nil, err
	}

	var dirs []InvoiceDirectoryReference

	for _, folderYear := range yearsDir {

		year := path.Base(folderYear)

		testYear, err := strconv.Atoi(year)

		if err != nil {
			log.Printf("invalid year folder: %s", folderYear)
			continue
		}

		monthsDir, err := h.driver.Ls(path.Join(folderInvoices, year))

		if err != nil {
			return nil, err
		}

		for _, monthFolder := range monthsDir {

			month := path.Base(monthFolder)

			testMonth, err := strconv.Atoi(month)

			if err != nil || testMonth < 1 || testMonth > 12 {
				log.Printf("invalid month folder: %s", monthFolder)
				continue
			}

			dirs = append(dirs, &directoryInvoice{
				year:   testYear,
				month:  time.Month(testMonth),
				driver: h.driver,
				folder: path.Join(folderInvoices, year, month),
			})
		}

	}

	return dirs, nil

}
func (h *handler) SdiSuccess(ctx context.Context, year int, month time.Month) (SDIResultDirectoryReference, error) {

	err := h.driver.MkdirAll(h.genPath(folderSDISuccess, year, month))

	if err != nil {
		return nil, err
	}

	return &sdiResultDirectoryReference{
		year:   year,
		month:  month,
		driver: h.driver,
		folder: folderSDISuccess,
	}, nil

}
func (h *handler) SdiFail(ctx context.Context, year int, month time.Month) (SDIResultDirectoryReference, error) {

	err := h.driver.MkdirAll(h.genPath(folderSDIFail, year, month))

	if err != nil {
		return nil, err
	}

	return &sdiResultDirectoryReference{
		year:   year,
		month:  month,
		driver: h.driver,
		folder: folderSDIFail,
	}, nil

}

func (h *handler) genPath(d string, y int, m time.Month) string {
	return path.Join(d, fmt.Sprintf("%d", y), fmt.Sprintf("%d", m))
}
