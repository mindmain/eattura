package os

import (
	"encoding/json"
	"fmt"

	"github.com/mindmain/eattura/internal/core"
	"github.com/mindmain/eattura/internal/secure/driver"
	"github.com/zalando/go-keyring"
)

type storageFromOs struct{}

func New() driver.Storage {
	return &storageFromOs{}
}

func (s *storageFromOs) Create(key string, creds *driver.Credentials) error {

	var book = make(driver.CredentialsBook)

	result, err := keyring.Get(core.AppId, core.AppName)

	if err != nil {
		if err != keyring.ErrNotFound {

			return err
		}
	} else {
		if err := json.Unmarshal([]byte(result), &book); err != nil {
			return err
		}
	}

	book[key] = creds

	data, err := json.Marshal(book)

	if err != nil {
		return err
	}

	return keyring.Set(core.AppId, core.AppName, string(data))

}
func (s *storageFromOs) Get(key string) (*driver.Credentials, error) {

	var book driver.CredentialsBook

	result, err := keyring.Get(core.AppId, core.AppName)

	if err != nil {
		return nil, err
	}

	if err := json.Unmarshal([]byte(result), &book); err != nil {
		return nil, err
	}

	if creds, ok := book[key]; ok {
		return creds, nil
	}

	return nil, fmt.Errorf("key not found")

}
func (s *storageFromOs) Delete(key string) error {

	var book driver.CredentialsBook

	result, err := keyring.Get(core.AppId, core.AppName)

	if err != nil {
		return err
	}

	if err := json.Unmarshal([]byte(result), &book); err != nil {
		return err
	}

	delete(book, key)

	data, err := json.Marshal(book)

	if err != nil {
		return err
	}

	return keyring.Set(core.AppId, core.AppName, string(data))

}
