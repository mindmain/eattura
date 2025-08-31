package main

import (
	"github.com/mindmain/eattura/internal/db"
	"github.com/spf13/cobra"
)

func DatabaseCommand() *cobra.Command {
	cmd := &cobra.Command{
		Use: "db",
	}

	cmd.AddCommand(databaseCommandInit())
	cmd.AddCommand(commandCount())
	return cmd
}

func databaseCommandInit() *cobra.Command {
	cmd := &cobra.Command{
		Use: "init",
		Run: func(cmd *cobra.Command, args []string) {
			database, err := db.New()

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

			database, err := db.New()

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
