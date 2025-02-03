package core

import (
	"cmp"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"path"
)

func init() {
	var home string
	var err error
	if os.Getenv("HOME") == "" {
		home, err = os.UserHomeDir()

		if err != nil {
			log.Fatal(err)
		}
	} else {
		home = os.Getenv("HOME")
	}

	ConfigPath = path.Join(home, folderAppName, configFileName)

}

const (
	folderAppName  = ".eattura"
	configFileName = "config.json"
	folderInvoices = "invoices"
	folderSecure   = "secure"
	// this port derive: 'E' and 'A' Hexadecimal format form ASCII table
	DefaultPort = 4541
)

var ConfigPath = ""

// if set, the config file will not be loaded from %HOME%/.eattura/config.json
var (
	DisableConfigFile = cmp.Or(os.Getenv("EATTURA_DISABLE_CONFIG_FILE"), "")
)

type EnvVar string

func (e EnvVar) Get() string {
	return os.Getenv(string(e))
}

var (

	//if set will replace the default config value
	envVariableNameDatabaseType EnvVar = "EATTURA_DATABASE_TYPE"

	// if set will replace the default config value
	envVariableNameInvoiceFolder EnvVar = "EATTURA_INVOICE_FOLDER"

	// if set will replace the default config value, empty string is allowed ony if the config file is used
	// in that case the value will be loaded from the config file the default value is used ~/.eattura/invoices
	envVariableInvoiceDriver EnvVar = "EATTURA_INVOICE_DRIVER"

	// if set will replace the default config value for the secure folder will be used from driver
	// the default value is local, because the secure driver is user for the storage credentials of pec
	envVariableNameFolderSecure EnvVar = "EATTURA_FOLDER_SECURE"

	// if set false will disable the frontend server
	envVariableDisabledFrontend EnvVar = "EATTURA_DISABLED_FRONTEND"
)

// DriverConfig is a configuration for the driver used to store the invoices or secure files
// Type: the type of the driver type (local, s3, gcs)
// if the type is local the root is the path to the folder where the files will be stored
// if the type is s3 or gcs the root is the bucket name
// ServiceAccount is the path to the service account file for the gcs driver.
type DriverConfig struct {
	Type           string `json:"type"`
	Root           string `json:"root"`
	Bucket         string `json:"bucket,omitempty"`
	ServiceAccount string `json:"service_account,omitempty"`
}

type DatabaseConfig struct {
	Type         string `json:"type"`
	DatabaseName string `json:"database_name"`
	Uri          string `json:"uri"`
	User         string `json:"user,omitempty"`
	Pass         string `json:"pass,omitempty"`
}

type AppConfig struct {
	DisabledFrontend bool `json:"disabled_frontend,omitempty"`
}

// Config is the main configuration for the application
type Config struct {
	DatabaseType  DatabaseConfig `json:"database"`
	InvoiceDriver DriverConfig   `json:"invoice_driver"`
	SecureDriver  DriverConfig   `json:"secure_driver"`
	AppConfig     AppConfig      `json:"app_config"`
}

func defaultConfig() *Config {

	return &Config{
		DatabaseType: DatabaseConfig{
			Type: "sqlite3",
		},
		InvoiceDriver: DriverConfig{
			Type: "local",
			Root: path.Join(ConfigPath, configFileName, folderInvoices),
		},

		SecureDriver: DriverConfig{
			Type: "local",
			Root: path.Join(ConfigPath, configFileName, folderSecure),
		},
	}

}
func (c *Config) alignWithEnv() *Config {

	if envVariableNameDatabaseType.Get() != "" {
		c.DatabaseType.Type = envVariableNameDatabaseType.Get()
	}

	if envVariableNameInvoiceFolder.Get() != "" {
		c.InvoiceDriver.Type = envVariableNameInvoiceFolder.Get()
	}

	if envVariableInvoiceDriver.Get() != "" {
		c.InvoiceDriver.Root = envVariableInvoiceDriver.Get()
	}

	if envVariableNameFolderSecure.Get() != "" {
		c.SecureDriver.Type = envVariableNameFolderSecure.Get()
	}

	if envVariableDisabledFrontend.Get() == "false" {
		c.AppConfig.DisabledFrontend = false
	} else if envVariableDisabledFrontend.Get() == "true" {
		c.AppConfig.DisabledFrontend = true
	}

	return c

}

// Provide a default configuration, or load the configuration from the config file
// if [EATTURA_DISABLE_CONFIG_FILE] is unset, try to read configuration from the file
// located at ~/.eattura/config.json
// if the file does not exist, create it with the default configuration
// every field in the configuration can be overridden by the environment variable
func LoadConfig() *Config {

	if DisableConfigFile != "" {
		return defaultConfig().alignWithEnv()
	}

	if path.Ext(ConfigPath) == "" {
		ConfigPath = fmt.Sprintf("%s.json", ConfigPath)
	}

	config := &Config{}

	if _, err := os.Stat(ConfigPath); os.IsNotExist(err) {

		if err := os.MkdirAll(path.Dir(ConfigPath), 0755); err != nil {

			log.Fatal(err)
		}

		bb, err := json.MarshalIndent(defaultConfig(), "", "  ")

		if err != nil {
			log.Fatal(err)
		}

		err = os.WriteFile(ConfigPath, bb, 0644)
		if err != nil {
			panic(err)
		}
	}

	bb, err := os.ReadFile(ConfigPath)
	if err != nil {
		panic(err)
	}

	err = json.Unmarshal(bb, config)

	if err != nil {
		panic(err)
	}

	return config.alignWithEnv()

}
