package main

import (
	"fmt"

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
			println(core.ConfigPath)
		},
	}

	cmd.Flags().BoolVarP(&versionFlag, "version", "v", false, "Print version")

	cmd.PersistentFlags().StringVarP(&core.ConfigPath, "config", "c", core.ConfigPath, fmt.Sprintf("Path to the configuration file, default is %s", core.ConfigPath))
	cmd.AddCommand(core.Command())
	cmd.Execute()
}
