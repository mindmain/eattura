package main

import (
	"fmt"
	"log"

	"github.com/mindmain/eattura"
	"github.com/spf13/cobra"
)

var service eattura.Eattura

func CommandService() *cobra.Command {
	cmd := &cobra.Command{
		Use:     "service",
		Aliases: []string{"s"},
		Short:   "Service command",
		Run: func(cmd *cobra.Command, args []string) {
			println("Service command")
		},
		PersistentPreRun: func(cmd *cobra.Command, args []string) {

			srv, err := eattura.New()

			if err != nil {
				log.Fatalf("Error on create eattura/service: %v", err)
			}

			service = srv
		},
	}

	cmd.AddCommand(commandLoader())

	return cmd
}

func commandLoader() *cobra.Command {
	cmd := &cobra.Command{
		Use:     "load",
		Aliases: []string{"l"},
		Short:   "Loader command",
		Run: func(cmd *cobra.Command, args []string) {
			cmd.Help()
		},
	}

	cmd.AddCommand(commandLoaderFromPath())

	return cmd
}

func commandLoaderFromPath() *cobra.Command {

	var from string

	cmd := &cobra.Command{
		Use:   "path",
		Short: "Load invoice from path",

		Run: func(cmd *cobra.Command, args []string) {

			if from == "" {
				log.Fatal("from path is required")
			}

			if result, err := service.LoadInvoiceFromAnotherDirectory(cmd.Context(), from); err != nil {
				log.Fatalf("Error on load invoice from path: %v", err)
			} else {
				fmt.Printf("saved %d/%d invoices found (%d errors)\n", result.Loaded, result.Loaded, result.Failed)
			}

		},
	}

	cmd.Flags().StringVarP(&from, "from", "f", "", "Path to load invoice from")

	return cmd
}
