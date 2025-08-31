package fs

import (
	"io"
	"path"

	"github.com/mindmain/eattura/internal/fs/driver"
)

type invoiceReference struct {
	driver driver.FileDriver

	filepath string
}

func (i *invoiceReference) Name() string {
	return path.Base(i.filepath)
}
func (i *invoiceReference) Path() string {
	return path.Dir(i.filepath)
}
func (i *invoiceReference) Read() ([]byte, error) {
	file, err := i.driver.OpenFileRead(i.filepath)

	if err != nil {
		return nil, err
	}

	defer file.Close()

	bb, err := io.ReadAll(file)

	if err != nil {
		return nil, err
	}

	return bb, nil
}
func (i *invoiceReference) Write(data []byte) error {
	return i.driver.WriteFile(i.filepath, data)
}
