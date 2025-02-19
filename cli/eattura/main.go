package main

import (
	"fmt"
	"os"

	"github.com/mindmain/eattura/core"
	"github.com/spf13/cobra"
)

const (
	// version is the current version of the application, will be set by the build system
	version = "v0.0.1-local"
)

func main() {

	var versionFlag bool
	cmd := cobra.Command{
		Run: func(cmd *cobra.Command, args []string) {

			if versionFlag {
				cmd.Println(version)
			}
		},
	}

	cmd.Flags().BoolVarP(&versionFlag, "version", "v", false, "Print version")

	cmd.PersistentFlags().StringVarP(&core.ConfigPathFile, "config", "c", core.ConfigPathFile, fmt.Sprintf("Path to the configuration file, default is %s", core.ConfigPathFile))
	cmd.AddCommand(ConfigCommand())
	cmd.AddCommand(FsCommand())
	cmd.AddCommand(SecureCommand())
	cmd.AddCommand(PecCommand())
	cmd.AddCommand(DatabaseCommand())
	cmd.AddCommand(CommandService())
	cobra.OnInitialize(func() {
		if err := core.InitConfig(); err != nil {
			cmd.Println(err)
			os.Exit(1)
		}
	})
	cmd.Execute()
}
