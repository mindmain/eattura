//go:build local

package ld

import (
	"fmt"
	"io"
	"os"
	"path"

	"github.com/mindmain/eattura/fs/driver"
)

func init() {
	driver.Register("local", New)
}

type localFileDriver struct {
	root string
}

func New(config *driver.StorageConfig) (driver.FileDriver, error) {

	if _, err := os.Stat(config.Path); os.IsNotExist(err) {
		return nil, fmt.Errorf("path %s does not exist", config.Path)
	}

	return &localFileDriver{
		root: config.Path,
	}, nil
}

func (l *localFileDriver) CanWrite() bool {

	// check if the directory is writable

	file, err := os.CreateTemp(l.root, "test")

	if err != nil {
		return false
	}

	if err := file.Close(); err != nil {
		return false
	}

	if err := os.Remove(file.Name()); err != nil {
		return false
	}

	return true

}

func (l *localFileDriver) CanRead() bool {

	// check if the directory is readable

	if _, err := os.Stat(l.root); os.IsNotExist(err) {
		return false
	}

	_, err := os.ReadDir(l.root)

	return err == nil
}

func (l *localFileDriver) MkdirAll(directory string) error {

	dir := path.Join(l.root, directory)

	if _, err := os.Stat(dir); os.IsNotExist(err) {
		if err := os.MkdirAll(dir, os.ModePerm); err != nil {
			return err
		}
	}

	return nil

}
func (l *localFileDriver) RemoveAll(directory string) error {

	dir := path.Join(l.root, directory)
	return os.RemoveAll(dir)

}
func (l *localFileDriver) OpenFile(file string) (io.ReadWriteCloser, error) {

	f := path.Join(l.root, file)

	ff, err := os.OpenFile(f, os.O_RDWR|os.O_CREATE, os.ModePerm)

	if err != nil {
		return nil, err
	}

	return ff, nil

}
func (l *localFileDriver) OpenFileRead(file string) (io.ReadCloser, error) {

	filePath := path.Join(l.root, file)

	ff, err := os.Open(filePath)

	if err != nil {
		return nil, err
	}

	return ff, nil

}
func (l *localFileDriver) WriteFile(file string, data []byte) error {

	file = path.Join(l.root, file)

	ff, err := os.Create(file)

	if err != nil {
		return err
	}

	defer ff.Close()

	_, err = ff.Write(data)

	if err != nil {

		return err

	}

	return nil

}
func (l *localFileDriver) Ls(directory string) ([]string, error) {

	directory = path.Join(l.root, directory)

	if _, err := os.Stat(directory); os.IsNotExist(err) {
		return nil, err
	}

	ref, err := os.Lstat(directory)

	if err != nil {
		return nil, err
	}

	if !ref.IsDir() {
		return nil, fmt.Errorf("path %s is not a directory", directory)
	}

	files, err := os.ReadDir(directory)

	if err != nil {
		return nil, err
	}

	var list []string

	for _, f := range files {
		list = append(list, path.Join(directory, f.Name()))
	}

	return list, nil

}
