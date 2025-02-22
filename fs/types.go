package fs

import (
	"context"
	"time"

	"github.com/mindmain/eattura/core"
	"github.com/mindmain/eattura/fs/driver"
	"github.com/mindmain/eattura/sdi/fe"
)

type Type = core.StorageType

const (
	Local Type = core.StorageLocalDirectory
	None  Type = core.StorageNone
)

const (
	folderInvoices   string = "invoices"
	folderSDISuccess string = "sdi_success"
	folderSDIFail    string = "sdi_fail"
	folderDraft      string = "draft"
)

type FileReference interface {
	Name() string
	Path() string
	Read() ([]byte, error)
	Write(data []byte) error
}

type DirectoryReference interface {
	Path() string
	Year() int
	Month() time.Month
}

type InvoiceDirectoryReference interface {
	DirectoryReference
	Invoices(ctx context.Context) ([]FileReference, error)
}

type SDIResultDirectoryReference interface {
	DirectoryReference
	Write(filename string, data []byte) error
	Files() []string
}

type InvoiceFileHandler interface {
	ReadInvoice(ctx context.Context, year, month int, filename string) (*fe.FatturaElettronica, error)
	SaveInvoice(ctx context.Context, filename string, invoice *fe.FatturaElettronica) error
	List(ctx context.Context) ([]InvoiceDirectoryReference, error)

	SdiSuccess(ctx context.Context, year int, month time.Month) (SDIResultDirectoryReference, error)
	SdiFail(ctx context.Context, year int, month time.Month) (SDIResultDirectoryReference, error)
}

type handler struct {
	driver driver.FileDriver
}

func New() (InvoiceFileHandler, error) {

	driverType := core.GetStorageType()

	dd, err := getDriverFromType(driverType)

	if err != nil {
		return nil, err
	}

	return &handler{
		driver: dd,
	}, nil

}

func getDriverFromType(driverType Type) (driver.FileDriver, error) {

	switch driverType {
	case Local:
		folder := core.Get("storage.folder")

		caller, err := driver.GetDriver("local")

		if err != nil {
			return nil, err
		}

		dd, err := caller(&driver.StorageConfig{
			Path: folder,
		})

		if err != nil {
			return nil, err
		}

		if !dd.CanRead() {
			return nil, ErrorPermissionDenied
		}

		if !dd.CanWrite() {
			return nil, ErrorPermissionDenied
		}

		return dd, nil
	case None:
		caller, _ := driver.GetDriver("none")
		dd, _ := caller(&driver.StorageConfig{})
		return dd, nil
	default:
		return nil, ErrorInvalidDriverType
	}

}
