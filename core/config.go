package core

import (
	"cmp"
	"fmt"
	"os"
	"path"
	"strings"

	"github.com/spf13/viper"
)

var ConfigPathFile = ""

// this port derive: 'E' and 'A' Hexadecimal format form ASCII table
var ApiPort = 4541

const folderAppName = ".eattura"

// if set, the config file will not loaded from $HOME/.eattura/config.json
var (
	isDisableConfigFile = cmp.Or(os.Getenv("EATTURA_DISABLE_CONFIG_FILE"), "")
)

func IsEnabledConfigFile() bool {
	return isDisableConfigFile == ""
}

func InitConfig() error {

	if ConfigPathFile == "" {
		home, err := os.UserHomeDir()

		if err != nil {
			return err
		}
		ConfigPathFile = path.Join(home, folderAppName, "config.json")
	}

	configPathFolder := path.Dir(ConfigPathFile)
	configFileName := path.Base(ConfigPathFile)

	if path.Ext(configFileName) != ".json" {
		return fmt.Errorf("config file must be a json file")
	}

	viper.SetConfigName(strings.TrimSuffix(configFileName, ".json"))
	viper.SetConfigType("json")

	viper.SetDefault("database.type", "sqlite3")
	viper.SetDefault("database.uri", path.Join(configPathFolder, "eattura.db"))
	viper.SetDefault("database.name", "eattura")
	viper.SetDefault("database.user", nil)
	viper.SetDefault("database.password", nil)

	viper.SetDefault("storage.type", "local")
	viper.SetDefault("storage.folder", path.Join(configPathFolder, "storage"))
	viper.SetDefault("secure.type", "os")

	viper.SetDefault("pec.username", nil)
	viper.SetDefault("pec.password", nil)
	viper.SetDefault("pec.sdi", nil)
	viper.SetDefault("pec.smtp.host", nil)
	viper.SetDefault("pec.smtp.port", 465)

	viper.SetDefault("pec.imap.host", nil)
	viper.SetDefault("pec.imap.port", 993)
	viper.SetDefault("pec.imap.inbox", "INBOX")
	viper.SetDefault("pec.imap.outbox", "INBOX/Sent")
	viper.SetDefault("pec.imap.tls", true)

	viper.SetDefault("issuer.type", "db")
	viper.SetDefault("issuer.title", nil)
	viper.SetDefault("issuer.name", nil)
	viper.SetDefault("issuer.surname", nil)
	viper.SetDefault("issuer.denomination", nil)
	viper.SetDefault("issuer.email", nil)
	viper.SetDefault("issuer.phone", nil)
	viper.SetDefault("issuer.cod_eori", nil)
	viper.SetDefault("issuer.vat_code", nil)
	viper.SetDefault("issuer.vat_nation", nil)
	viper.SetDefault("issuer.fiscal_code", nil)

	viper.SetDefault("issuer.address.country", nil)
	viper.SetDefault("issuer.address.province", nil)
	viper.SetDefault("issuer.address.city", nil)
	viper.SetDefault("issuer.address.street", nil)
	viper.SetDefault("issuer.address.number", nil)
	viper.SetDefault("issuer.address.zip_code", nil)

	viper.SetDefault("issuer.location.country", nil)
	viper.SetDefault("issuer.location.province", nil)
	viper.SetDefault("issuer.location.city", nil)
	viper.SetDefault("issuer.location.street", nil)
	viper.SetDefault("issuer.location.number", nil)
	viper.SetDefault("issuer.location.zip_code", nil)

	viper.SetDefault("issuer.rea.office", nil)
	viper.SetDefault("issuer.rea.number", nil)
	viper.SetDefault("issuer.rea.capital", nil)
	viper.SetDefault("issuer.rea.solo_shareholder", nil)
	viper.SetDefault("issuer.rea.in_liquidation", nil)

	viper.SetDefault("api.port", ApiPort)
	viper.SetDefault("api.enabled", false)
	viper.SetDefault("api.cors", []string{"*"})
	viper.SetDefault("api.cors.methods", []string{"GET", "POST", "PUT", "DELETE", "OPTIONS"})

	viper.SetEnvPrefix("EATTURA")

	for _, key := range viper.AllKeys() {
		keyResult := fmt.Sprintf("EATTURA_%s", strings.ToUpper(strings.ReplaceAll(key, ".", "_")))
		viper.BindEnv(key, keyResult)
	}

	if IsEnabledConfigFile() {

		viper.AddConfigPath(configPathFolder)
		if err := viper.ReadInConfig(); err != nil {
			if err := viper.SafeWriteConfig(); err != nil {
				return err
			}
		} else {
			if err := viper.WriteConfigAs(ConfigPathFile); err != nil {
				return err
			}
		}
	}

	return nil
}

func Get(key string) string {
	return viper.GetString(key)
}

func GetFloat(key string) float64 {
	return viper.GetFloat64(key)
}

func GetInt(key string) int {
	return viper.GetInt(key)
}

func GetBool(key string) bool {
	return viper.GetBool(key)
}
