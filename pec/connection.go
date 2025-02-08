package pec

import (
	"context"
	"fmt"

	"github.com/emersion/go-imap"
	"github.com/emersion/go-imap/client"
	"gopkg.in/gomail.v2"
)

type connection struct {
	clientImap *client.Client
	clientSmtp *gomail.Dialer
}

func (conn *connection) Boxes(ctx context.Context) ([]MBox, error) {

	var boxes []MBox

	var out = make(chan *imap.MailboxInfo, 10)
	if err := conn.clientImap.List("", "*", out); err != nil {
		return nil, err
	}

	for m := range out {
		boxes = append(boxes, &pecMailBox{
			name: m.Name,
			conn: conn,
		})
	}

	return boxes, nil
}
func (conn *connection) Send(ctx context.Context, msg *Message) error {
	return fmt.Errorf("not implemented")
}
func (conn *connection) Close() error {

	if conn.clientImap != nil {
		conn.clientImap.Close()
	}

	return nil
}

func (conn *connection) Box(ctx context.Context, name string) (MBox, error) {
	return &pecMailBox{
		name: name,
		conn: conn,
	}, nil
}
