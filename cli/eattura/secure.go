package main

import (
	"encoding/json"

	"github.com/mindmain/eattura/internal/core"
	"github.com/mindmain/eattura/internal/secure"
	"github.com/spf13/cobra"
)

func SecureCommand() *cobra.Command {
	cmd := cobra.Command{
		Use:   "secure",
		Short: "Manage the secure credentials",
	}

	cmd.AddCommand(secureCreateCommand())
	cmd.AddCommand(secureGetCommand())
	cmd.AddCommand(secureDeleteCommand())

	return &cmd
}

func secureCreateCommand() *cobra.Command {
	cmd := cobra.Command{
		Use:   "create",
		Short: "Create a new secure credentials",
		Run: func(cmd *cobra.Command, args []string) {

			if len(args) != 1 {
				cmd.Println("key is required")
				return
			}

			var creds secure.Credentials

			ss, err := secure.New()

			if err != nil {
				cmd.Println(err)
				return
			}

			err = ss.Create(args[0], &creds)

			if err != nil {
				cmd.Println(err)
				return
			}

			cmd.Println("done")

		},
	}

	return &cmd
}

func secureGetCommand() *cobra.Command {
	cmd := cobra.Command{
		Use:   "get",
		Short: "Get the secure credentials",
		Run: func(cmd *cobra.Command, args []string) {

			t := secure.Type(core.Get("secure.type"))
			var key string
			if len(args) != 1 && t == secure.Os {
				cmd.Println("key is required")
				return
			} else {
				key = args[0]
			}

			ss, err := secure.New()

			if err != nil {
				cmd.Println(err)
				return
			}

			if t == secure.Os {
				// TODO implement the request prompt method and ask for every key of credentials
			}

			creds, err := ss.Get(key)

			if err != nil {
				cmd.Println(err)
				return
			}
			res, err := json.MarshalIndent(creds, "", "  ")

			if err != nil {
				cmd.Println(err)
				return
			}

			cmd.Println(string(res))

		},
	}

	return &cmd
}

func secureDeleteCommand() *cobra.Command {
	cmd := cobra.Command{
		Use:   "delete",
		Short: "Delete the secure credentials",
		Run: func(cmd *cobra.Command, args []string) {

			if len(args) != 1 {
				cmd.Println("key is required")
				return
			}

			key := args[0]

			ss, err := secure.New()

			if err != nil {
				cmd.Println(err)
				return
			}

			err = ss.Delete(key)

			if err != nil {
				cmd.Println(err)
				return
			}

			cmd.Println("done")

		},
	}

	return &cmd
}
