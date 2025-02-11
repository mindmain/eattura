package driver

import (
	"fmt"
	"io"
)

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
