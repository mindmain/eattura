package fs

import (
	"path"

	"github.com/spf13/cobra"
)

func Command() *cobra.Command {

	cmd := &cobra.Command{
		Use:   "fs",
		Short: "utilities for read invoices file.",
		Run: func(cmd *cobra.Command, args []string) {
			cmd.Help()
		},
	}

	cmd.AddCommand(commandInvoice())

	return cmd
}

func commandInvoice() *cobra.Command {

	cmd := &cobra.Command{
		Use:     "invoice",
		Aliases: []string{"inv", "i", "fe"},
		Short:   "utilities for read invoices file.",
	}

	cmd.AddCommand(commandInvoiceLs())

	return cmd
}

func commandInvoiceLs() *cobra.Command {

	cmd := &cobra.Command{
		Use:   "ls",
		Short: "list all invoices.",
		Run: func(cmd *cobra.Command, args []string) {

			storage, err := New()

			if err != nil {
				cmd.PrintErr(err)
				return
			}

			dirs, err := storage.List(cmd.Context())

			if err != nil {
				cmd.PrintErr(err)
				return
			}

			if len(dirs) == 0 {
				println("no invoices found.")
				return
			}

			for _, dir := range dirs {
				println(dir.Path())
				invoices, err := dir.Invoices(cmd.Context())

				if err != nil {
					cmd.PrintErr(err)
					return
				}

				for _, invoice := range invoices {
					println(path.Join(dir.Path(), invoice.Name()))
				}
			}
		},
	}

	return cmd
}
