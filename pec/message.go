package pec

import (
	"bufio"
	"bytes"
	"fmt"
	"io"
	"log"
	"mime"
	"mime/multipart"
	"net/mail"
	"strings"
	"time"

	"github.com/emersion/go-imap"
	"github.com/mindmain/eattura/sdi"
	"github.com/pkg/errors"
)

const maxDepthMultiParts = 10

var ErrMaxDepthMultiParts = fmt.Errorf("max depth multipart reached")
var ErrContentTypeNotSupported = fmt.Errorf("Content-Type not supported")
var ErrParsingContent = fmt.Errorf("error parsing")

type Attachment struct {
	Name        string
	ContentType string
	Data        []byte
}

func (a *Attachment) String() string {
	return fmt.Sprintf("📎 %s %s (%d bytes)", a.Name, a.ContentType, len(a.Data))
}

type Message struct {
	conn        *connection
	imapMessage *imap.Message
	uid         uint32
	Type        MessageType   `json:"type"`
	At          time.Time     `json:"at"`
	Subject     string        `json:"subject"`
	Attachments []*Attachment `json:"attachments"`
}

func (m *Message) String() string {

	return fmt.Sprintf("\n 📥 [%s 🕒] | %s | %s %s", m.At, m.Type, m.Subject, func() string {

		if len(m.Attachments) > 0 {
			s := new(bytes.Buffer)
			for index, a := range m.Attachments {

				if index == len(m.Attachments)-1 {
					s.WriteString("\n└─")
				} else {
					s.WriteString("\n├─")
				}

				s.WriteString(a.String())
			}
			return s.String()
		}

		return "\n└─ No attachments"

	}())
}

func (m *Message) GetInvoice() (*Attachment, bool) {

	for _, a := range m.Attachments {
		if sdi.IsInvoiceFileName(a.Name) || sdi.IsInvoiceFileNameCrypt(a.Name) {
			return a, true
		}
	}

	return nil, false
}

// Resolve an mail.Message to extract attachments, and resolve every attachments is a multipart/
func (m *Message) resolveAttachments(email *mail.Message) error {

	mediaType, params, err := mime.ParseMediaType(email.Header.Get("Content-Type"))

	if err != nil {
		return errors.Wrapf(ErrParsingContent, "Content-Type: %v", err)
	}

	if strings.HasPrefix(mediaType, "multipart/") && params["boundary"] != "" {

		mr := multipart.NewReader(email.Body, params["boundary"])
		return m.multipartReader(mr, 0)
	}

	return nil

}

func (m *Message) multipartReader(part *multipart.Reader, deep int) error {

	for {
		part, err := part.NextPart()

		if err == io.EOF {
			break
		}

		if err != nil {
			return errors.Wrapf(ErrParsingContent, "part: %v", err)
		}

		mediaType, params, err := mime.ParseMediaType(part.Header.Get("Content-Type"))

		if err != nil {
			return errors.Wrapf(ErrParsingContent, "Content-Type: %v", err)
		}

		if strings.HasPrefix(mediaType, "multipart/") && params["boundary"] != "" {

			if deep > maxDepthMultiParts {
				return ErrMaxDepthMultiParts
			}

			bb := bufio.NewReader(part)
			reader := multipart.NewReader(bb, params["boundary"])

			if err := m.multipartReader(reader, deep+1); err != nil {
				return err
			}
		} else {

			switch mediaType {

			case "message/rfc822":
				data, err := io.ReadAll(part)

				if err != nil {
					return fmt.Errorf("error reading part message/rfc822: %v", err)
				}

				mailContent, err := mail.ReadMessage(strings.NewReader(string(data)))

				if err != nil {
					return fmt.Errorf("error reading message/rfc822: %v", err)
				}

				if err := m.resolveAttachments(mailContent); err != nil {
					return fmt.Errorf("error resolving attachments: %v", err)
				}

			default:
				if params["name"] == "" {
					continue
				}

				attachment := &Attachment{
					Name:        params["name"],
					ContentType: mediaType,
				}

				data, err := io.ReadAll(part)

				if err != nil {
					return fmt.Errorf("error reading part: %v", err)
				}

				attachment.Data = data

				m.Attachments = append(m.Attachments, attachment)
			}

		}
	}

	return nil
}

func parseMessage(msg *imap.Message, conn *connection) *Message {

	subject := msg.Envelope.Subject

	message := &Message{
		conn:        conn,
		imapMessage: msg,
		uid:         msg.Uid,

		Type:    determinateTypeMessage(subject),
		At:      msg.Envelope.Date,
		Subject: subject,
	}

	if msg.Body != nil {

		for _, part := range msg.Body {

			mail, err := mail.ReadMessage(part)

			if err != nil {
				log.Println("Error reading message:", err)
				continue
			}

			if err := message.resolveAttachments(mail); err != nil {
				log.Println("Error resolving attachments:", err)
			}
		}
	}

	return message

}
