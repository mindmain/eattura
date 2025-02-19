package eattura

import (
	"context"
	"errors"
	"log"
	"os"
	"path"
	"strings"
	"sync"

	"github.com/mindmain/eattura/core"
	"github.com/mindmain/eattura/db"
	"github.com/mindmain/eattura/fs"
	"github.com/mindmain/eattura/pec"
	"github.com/mindmain/eattura/sdi"
	"github.com/mindmain/eattura/sdi/fe"
)

type ResultLoader struct {
	Found  int
	Loaded int
	Failed int
}

type Loader interface {
	// LoadInvoiceFromDirectory load all the invoice from the directory search inside the path. and subpath.
	// when found an XML file try to parse it and create the invoice, if the invoice is already present
	// copy file on fs and update/create the invoice on db.
	// if isn't possible to parse the file, ignore file and log the error.
	LoadInvoiceFromAnotherDirectory(ctx context.Context, path string) (*ResultLoader, error)

	// LoadFromPec load all the invoice from the PEC service, and create the invoice on the db.
	// if the invoice is already present, update the invoice.
	// if the invoice is already present and the status is final, ignore the invoice.
	// Download the file from the PEC service and save it on the fs.
	// Read Inbound and Outbound invoice.
	LoadFromPec(ctx context.Context) (*ResultLoader, error)

	// Realign check every invoice on the storage and check if the invoice is present on the db.
	// if the invoice is present on the storage and not present on the db, create the invoice with status [paid or received].
	// (the status will be edited on the future, by user but is initial considered payed).
	// the status is determinate from CedentePrestatore or CessionarioCommittente block on XML file.
	// if invoice not has issuer into CedentePrestatore or CessionarioCommittente block, ignore the invoice.
	Realign(ctx context.Context, issuer *Issuer) error

	// Check count all the invoice on the db and check if the invoice is present on the storage.
	// return [ErrInconsistentData] if the invoices on db are different from the storage.
	// if found an invoice on the storage and not present on the db, return error.
	Check(ctx context.Context) error
}

type defaultLoader struct {
	store         fs.InvoiceFileHandler
	db            db.Database
	pec           pec.Client
	issuerHandler IssuerHandler
}

func (l *defaultLoader) LoadInvoiceFromAnotherDirectory(ctx context.Context, folder string) (*ResultLoader, error) {

	var lock = &sync.Mutex{}
	var wg = &sync.WaitGroup{}
	worker := 3
	//Check if the path is a directory or not and check is tha path is same config path
	var result = &ResultLoader{}
	stat, err := os.Stat(folder)

	if err != nil {
		return nil, err
	}

	if os.IsNotExist(err) {
		return nil, errors.New("path not exist")
	}

	if !stat.IsDir() {
		return nil, errors.New("path is not a directory")
	}

	//Check if the path is the same of the config path
	if folder == core.Get("storage.folder") || strings.HasSuffix(folder, core.Get("storage.folder")) {
		return nil, errors.New("path is the same of the config path")
	}

	items, err := os.ReadDir(folder)

	if err != nil {
		return nil, err
	}

	var files = make(chan *fe.FatturaElettronica, 10)

	wg.Add(worker)

	for range worker {
		go func() {
			defer wg.Done()
			for invoice := range files {
				filename, _ := invoice.Filename()
				if err := l.store.SaveInvoice(ctx, filename, invoice); err != nil {
					log.Printf("can't save invoice: %s - %v", filename, err)
					lock.Lock()
					result.Failed++
					lock.Unlock()
					continue
				} else {
					lock.Lock()
					result.Loaded++
					lock.Unlock()
				}

			}
		}()
	}

	for _, item := range items {
		if item.IsDir() {
			if resultSub, err := l.LoadInvoiceFromAnotherDirectory(ctx, path.Join(folder, item.Name())); err != nil {
				log.Printf("can't load: %s - %v", path.Join(folder, item.Name()), err)
				continue
			} else {
				lock.Lock()
				result.Found += resultSub.Found
				result.Loaded += resultSub.Loaded
				result.Failed += resultSub.Failed
				lock.Unlock()
			}
		} else {
			if strings.ToLower(path.Ext(item.Name())) == ".xml" {

				if sdi.IsInvoiceFileName(item.Name()) || sdi.IsInvoiceFileNameCrypt(item.Name()) {
					result.Found++
					file, err := os.OpenFile(path.Join(folder, item.Name()), os.O_RDONLY, 0666)

					if err != nil {
						log.Printf("can't read: %s - %v", path.Join(folder, item.Name()), err)
						result.Failed++
						continue
					}

					invoice, err := sdi.Read(item.Name(), file)

					if err != nil {
						log.Printf("can't parse: %s - %v", path.Join(folder, item.Name()), err)
						result.Failed++
						continue
					}
					files <- invoice
				}
			}
		}

	}

	close(files)
	wg.Wait()
	return result, nil

}
func (l *defaultLoader) LoadFromPec(ctx context.Context) (*ResultLoader, error) {
	return &ResultLoader{}, nil
}
func (l *defaultLoader) Realign(ctx context.Context, issuer *Issuer) error {
	return nil
}
func (l *defaultLoader) Check(ctx context.Context) error {
	return nil
}
