package fs

import (
	"fmt"
	"path"
	"time"

	"github.com/mindmain/eattura/internal/fs/driver"
)

type sdiResultDirectoryReference struct {
	driver driver.FileDriver
	year   int
	month  time.Month
	folder string
}

func (s *sdiResultDirectoryReference) Year() int {
	return s.year
}

func (s *sdiResultDirectoryReference) Month() time.Month {
	return s.month
}

func (s *sdiResultDirectoryReference) genPath(year int, month time.Month) string {
	return path.Join(s.folder, fmt.Sprintf("%d", year), fmt.Sprintf("%d", month))
}

func (s *sdiResultDirectoryReference) Path() string {
	return s.genPath(s.year, s.month)
}

func (s *sdiResultDirectoryReference) Write(filename string, data []byte) error {

	f := path.Join(s.genPath(s.year, s.month), filename)
	return s.driver.WriteFile(f, data)
}

func (s *sdiResultDirectoryReference) Files() []string {
	files, _ := s.driver.Ls(s.genPath(s.year, s.month))
	return files
}
