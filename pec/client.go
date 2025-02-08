package pec

import (
	"context"
	"fmt"

	"github.com/emersion/go-imap/client"
	"github.com/mindmain/eattura/secure"
	"gopkg.in/gomail.v2"
)

type clientPec struct {
	cred secure.Storage
}

func (c *clientPec) Connect(ctx context.Context, uuid string) (PEC, error) {

	credentials, err := c.cred.Get(uuid)

	if err != nil {
		return nil, err
	}

	imapConnection, err := newImapConnection(credentials)

	if err != nil {
		return nil, err
	}

	smtpConnection, err := newSmtpConnection(credentials)

	if err != nil {
		return nil, err
	}

	return &connection{
		clientImap: imapConnection,
		clientSmtp: smtpConnection,
		cred:       credentials,
	}, nil
}

func newImapConnection(credentials *secure.Credentials) (*client.Client, error) {

	conn, err := client.DialTLS(fmt.Sprintf("%s:%d", credentials.ImapHost, credentials.ImapPort), nil)

	if err != nil {
		return nil, err
	}

	if err := conn.Login(credentials.Username, credentials.Password); err != nil {
		return nil, err
	}

	return conn, nil
}

func newSmtpConnection(credentials *secure.Credentials) (*gomail.Dialer, error) {

	d := gomail.NewDialer(credentials.SmtpHost, credentials.SmtpPort, credentials.Username, credentials.Password)

	return d, nil

}
