package driver

import (
	"fmt"
	"io"
)

func init() {
	Register("none", func(cfg *StorageConfig) (FileDriver, error) {
		return &noneDriver{}, nil
	})
}

type FileDriver interface {
	CanRead() bool
	CanWrite() bool

	MkdirAll(path string) error
	RemoveAll(path string) error
	OpenFile(path string) (io.ReadWriteCloser, error)
	OpenFileRead(path string) (io.ReadCloser, error)
	WriteFile(path string, data []byte) error
	Ls(path string) ([]string, error)
}

type StorageConfig struct {
	Path string
}

var drivers = make(map[string]func(*StorageConfig) (FileDriver, error))

func Register(name string, driver func(*StorageConfig) (FileDriver, error)) {

	drivers[name] = driver

}

func GetDriver(name string) (func(*StorageConfig) (FileDriver, error), error) {

	if f, ok := drivers[name]; ok {
		return f, nil
	}

	return nil, fmt.Errorf("storage driver not found %s", name)

}

type noneDriver struct{}

func (nd *noneDriver) CanRead() bool {
	return false
}

func (nd *noneDriver) CanWrite() bool {
	return false
}

func (nd *noneDriver) MkdirAll(path string) error {
	return fmt.Errorf("driver is none, can't create folder")
}

func (nd *noneDriver) RemoveAll(path string) error {
	return fmt.Errorf("driver is none, can't remove folder")
}

func (nd *noneDriver) OpenFile(path string) (io.ReadWriteCloser, error) {
	return nil, fmt.Errorf("driver is none, can't open file")
}

func (nd *noneDriver) OpenFileRead(path string) (io.ReadCloser, error) {
	return nil, fmt.Errorf("driver is none, can't open file")
}

func (nd *noneDriver) WriteFile(path string, data []byte) error {
	return fmt.Errorf("driver is none, can't write file")
}

func (nd *noneDriver) Ls(path string) ([]string, error) {
	return nil, fmt.Errorf("driver is none, can't list folder")
}
