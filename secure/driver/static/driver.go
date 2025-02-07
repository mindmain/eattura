package static

import (
	"fmt"

	"github.com/mindmain/eattura/core"
	"github.com/mindmain/eattura/secure/driver"
)

func New() driver.Storage {
	return &staticSecure{}
}

type staticSecure struct{}

func (s *staticSecure) Create(_ string, _ *driver.Credentials) error {
	return fmt.Errorf("the configuration is static, cannot create see documentation for more information")
}
func (s *staticSecure) Get(_ string) (*driver.Credentials, error) {

	var creds = driver.Credentials{
		ImapHost:   core.Get("pec.imap.host"),
		ImapPort:   core.GetInt("pec.imap.port"),
		ImapInbox:  core.Get("pec.imap.inbox"),
		ImapOutbox: core.Get("pec.imap.outbox"),
		ImapTls:    core.GetBool("pec.imap.tls"),

		SmtpHost: core.Get("pec.smtp.host"),
		SmtpPort: core.GetInt("pec.smtp.port"),

		PecSdi: core.Get("pec.sdi"),

		Username: core.Get("pec.username"),
		Password: core.Get("pec.password"),
	}

	return &creds, nil

}
func (s *staticSecure) Delete(key string) error {
	return fmt.Errorf("the configuration is static, cannot delete see documentation for more information")
}
