package secure

import (
	"fmt"

	"github.com/mindmain/eattura/internal/core"
	"github.com/mindmain/eattura/internal/secure/driver"
	"github.com/mindmain/eattura/internal/secure/driver/os"
	"github.com/mindmain/eattura/internal/secure/driver/static"
)

type Type string

const (
	// Os is a storage type that stores the credentials in the secure credentials store of the operating system.
	// use github.com/zalando/go-keyring for supporting multiple operating systems.
	Os Type = "os"
	// Static is a storage type that does not store any data on disk, it is used for kubernetes secrets context
	// docker containers, etc.
	// you must define the credentials in the environment variables or in the configuration file.
	Static Type = "static"
)

var ErrInvalidStorageType = fmt.Errorf("invalid storage type")

type Storage = driver.Storage
type Credentials = driver.Credentials

func New() (Storage, error) {

	storageType := Type(core.Get("secure.type"))

	switch storageType {
	case Os:
		return os.New(), nil
	case Static:
		return static.New(), nil
	}

	return nil, ErrInvalidStorageType

}
