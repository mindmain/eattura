package db

import (
	"fmt"

	"github.com/mindmain/eattura/core"
	"github.com/mindmain/eattura/db/driver"
	"github.com/spf13/cobra"
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

func Command() *cobra.Command {
	cmd := &cobra.Command{
		Use: "db",
	}

	cmd.AddCommand(commandInit())
	cmd.AddCommand(commandCount())
	return cmd
}

func commandInit() *cobra.Command {
	cmd := &cobra.Command{
		Use: "init",
		Run: func(cmd *cobra.Command, args []string) {
			database, err := New()

			if err != nil {
				cmd.Println(err)
				return
			}

			err = database.Init()

			if err != nil {
				cmd.Println(err)
				return
			}

		},
	}

	return cmd
}

func commandCount() *cobra.Command {
	cmd := &cobra.Command{
		Use: "count",
		Run: func(cmd *cobra.Command, args []string) {

			database, err := New()

			if err != nil {
				cmd.Println(err)
				return
			}

			contactCount, err := database.Contact().Count(cmd.Context(), nil)

			if err != nil {
				cmd.Println(err)
				return
			}

			cmd.Println("Contact count:", contactCount)

		},
	}

	return cmd
}
