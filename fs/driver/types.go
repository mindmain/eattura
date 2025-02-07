package driver

import (
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
