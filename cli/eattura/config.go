package main

import (
	"encoding/json"
	"fmt"
	"log"
	"os"

	"github.com/mindmain/eattura/core"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func ConfigCommand() *cobra.Command {

	cmd := &cobra.Command{
		Use:   "config",
		Short: "init an configuration file",
	}

	cmd.AddCommand(configCommandGet())
	cmd.AddCommand(configCommandDelete())
	cmd.AddCommand(configCommandInit())
	return cmd
}

func configCommandGet() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "get",
		Short: "print the configuration",
		Run: func(cmd *cobra.Command, args []string) {

			if core.IsEnabledConfigFile() {
				config := viper.GetViper().ConfigFileUsed()
				fmt.Println("config file:", config)
			} else {
				fmt.Println("config file is disabled")
			}
			bb, err := json.MarshalIndent(viper.AllSettings(), "", "  ")

			if err != nil {
				log.Fatal(err)
			}

			fmt.Println(string(bb))
		},
	}

	return cmd
}

func configCommandDelete() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "delete",
		Short: "delete the configuration",
		Run: func(cmd *cobra.Command, args []string) {
			if _, err := os.Stat(core.ConfigPathFile); os.IsNotExist(err) {
				log.Fatal("config file does not exist")
			}

			err := os.Remove(core.ConfigPathFile)
			if err != nil {
				log.Fatal(err)
			}
		},
	}

	return cmd
}

func configCommandInit() *cobra.Command {

	cmd := &cobra.Command{
		Use:   "init",
		Short: "init an configuration file",

		Run: func(cmd *cobra.Command, args []string) {
			if !core.IsEnabledConfigFile() {
				log.Fatal("config file is disabled")
			}

			viper.SafeWriteConfig()
		},
	}

	return cmd

}
