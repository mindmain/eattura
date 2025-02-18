package db

import (
	"fmt"

	"github.com/mindmain/eattura/core"
	"github.com/mindmain/eattura/db/driver"
)

type Type string

const (
	TypeSqlite Type = "sqlite3"
)

type Database = driver.Database

func New() (driver.Database, error) {

	tt := getType(core.Get("database.type"))

	switch tt {
	case TypeSqlite:

		pathDb := core.Get("database.uri")
		caller, err := driver.GetDriver("sqlite3")

		if err != nil {
			return nil, err
		}

		database, err := caller(&driver.DatabaseConfig{
			Uri: pathDb,
		})

		if err != nil {
			return nil, err
		}

		return database, nil
	}

	return nil, fmt.Errorf("database type not supported")
}

func getType(raw string) Type {

	switch raw {
	case "sqlite3":
		return TypeSqlite
	}

	return ""
}
