package pec

import (
	"context"
	"strings"
	"time"

	"github.com/mindmain/eattura/secure"
)

type MessageType string

const (
	// Invoice declined from sdi
	MessageTypeFail MessageType = "fail"
	// Invoice from sdi
	MessageTypeInvoice MessageType = "invoice"
	// Success accepted invoice from sdi
	MessageTypeSuccess MessageType = "success"
	// Invoice sent
	MessageTypeSent MessageType = "sent"

	MessageTypeUnknown MessageType = "unknown"
)

func determinateTypeMessage(subject string) MessageType {
	if strings.Contains(subject, "Invio File") {
		return MessageTypeInvoice
	}

	if strings.Contains(subject, "Notifica di scarto") {
		return MessageTypeFail
	}

	if strings.Contains(subject, "Ricevuta di consegna") {
		return MessageTypeSuccess
	}

	return MessageTypeUnknown
}

type SearchMessage struct {
	From time.Time
	To   time.Time
	Tail int
}

type MBox interface {
	// ReadAll reads all the data from the PEC. this method should be called one time only.
	// but user can call for complete data refresh and check.
	ReadAll(ctx context.Context) (<-chan *Message, error)
	Read(ctx context.Context, request *SearchMessage) (<-chan *Message, error)
	Name() string
}

type PEC interface {
	Boxes(ctx context.Context) ([]MBox, error)
	Box(ctx context.Context, name string) (MBox, error)
	Send(ctx context.Context, msg *Message) error
	Close() error
}

type Client interface {
	Connect(ctx context.Context, uuid string) (PEC, error)
}

func NewPEC(ss secure.Storage) Client {

	return &clientPec{
		cred: ss,
	}
}
