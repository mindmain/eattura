package core

import (
	"encoding/json"
	"fmt"
	"log"
	"os"

	"github.com/spf13/cobra"
)

func Command() *cobra.Command {

	cmd := &cobra.Command{
		Use:   "config",
		Short: "init an configuration file",
	}

	cmd.AddCommand(commandInit())
	cmd.AddCommand(commandGet())
	cmd.AddCommand(commandDelete())

	return cmd
}

func commandInit() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "init",
		Short: "init an configuration file",
		Run: func(cmd *cobra.Command, args []string) {
			LoadConfig()
		},
	}

	return cmd
}

func commandGet() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "get",
		Short: "print the configuration",
		Run: func(cmd *cobra.Command, args []string) {

			if _, err := os.Stat(ConfigPath); os.IsNotExist(err) {
				log.Fatal("config file does not exist")
			}

			config := LoadConfig()

			bb, err := json.MarshalIndent(config, "", "  ")
			if err != nil {
				log.Fatal(err)
			}

			fmt.Println(string(bb))
		},
	}

	return cmd
}

func commandDelete() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "delete",
		Short: "delete the configuration",
		Run: func(cmd *cobra.Command, args []string) {
			if _, err := os.Stat(ConfigPath); os.IsNotExist(err) {
				log.Fatal("config file does not exist")
			}

			err := os.Remove(ConfigPath)
			if err != nil {
				log.Fatal(err)
			}
		},
	}

	return cmd
}
