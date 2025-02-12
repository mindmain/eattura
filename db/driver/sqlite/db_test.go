//go:build sqlite

package sqlite

import (
	"testing"

	"github.com/mindmain/eattura/db/driver"
)

var db driver.Database

func TestMain(t *testing.M) {
	var err error
	db, err = New(&driver.DatabaseConfig{
		Uri: "file::memory:?cache=shared",
	})

	if err != nil {
		panic(err)
	}

	if err := db.Init(); err != nil {
		panic(err)
	}

	t.Run()
	db.Close()
}
