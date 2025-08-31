package pec

import (
	"context"
	"fmt"
	"io"
	"log"
	"strings"

	"github.com/emersion/go-imap"
	"github.com/emersion/go-imap/client"
	"github.com/mindmain/eattura/internal/sdi"
	"github.com/mindmain/eattura/internal/secure"
	"gopkg.in/gomail.v2"
)

type connection struct {
	clientImap *client.Client
	clientSmtp *gomail.Dialer

	cred *secure.Credentials
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
func (conn *connection) Send(ctx context.Context, request *RequestSend) error {

	if request == nil {
		log.Println("can't send RequestSend is nil")
		return nil
	}

	if conn.cred.PecSdi == "" {
		return fmt.Errorf("missing recipient")
	}

	if !strings.HasSuffix(conn.cred.PecSdi, domainSdiEmail) {
		log.Println("[WARN] this destination is not a SDI pec/email (ignore this warning if you are debug/testing)")
	}

	m := gomail.NewMessage()
	m.SetHeader("From", conn.clientSmtp.Username)
	m.SetHeader("To", conn.cred.PecSdi)
	m.SetHeader("Subject", request.Subject)

	if len(request.Attachments) == 0 {
		log.Println("[WARN] no attachments found")
	}
	for _, attach := range request.Attachments {

		if attach.Name == "" {
			log.Println("[SKIP] attachment name is empty")
			continue
		}

		if len(attach.Data) == 0 {
			log.Printf("[SKIP] attachment %s data is empty", attach.Name)
			continue
		}

		if len(attach.ContentDisposition) == 0 {
			log.Println("[WARN] attachment content disposition is empty will use attachment style")
			attach.ContentDisposition = []string{fmt.Sprintf("attachment; filename=%s", attach.Name)}
		}

		if len(attach.ContentType) == 0 {
			log.Println("[WARN] attachment content type is empty will use application/octet-stream")
			attach.ContentType = []string{"application/octet-stream"}
		}

		if strings.HasSuffix(attach.Name, ".xml") {
			if attach.ContentType[0] != "application/xml" && attach.ContentType[0] != "text/xml" && attach.ContentType[0] != "application/octet-stream" {
				log.Println("[WARN] attachment content type is not xml will use application/xml")
				attach.ContentType = []string{"application/xml"}
			}

			if !sdi.IsInvoiceFileName(attach.Name) {
				log.Println("[SKIP] attachment name is not valid invoice name")
				continue
			}
		}

		if strings.HasSuffix(attach.Name, ".p7m") {
			if attach.ContentType[0] != "application/pkcs7-mime" {
				log.Println("[WARN] attachment content type is not p7m will use application/pkcs7-mime")
				attach.ContentType = []string{"application/pkcs7-mime"}
			}

			if !sdi.IsInvoiceFileNameCrypt(attach.Name) {
				log.Println("[SKIP] attachment name is not valid invoice name")
				continue
			}
		}

		m.Attach(attach.Name,
			gomail.SetHeader(map[string][]string{
				"Content-Type":        attach.ContentType,
				"Content-Disposition": attach.ContentDisposition,
			}),
			gomail.SetCopyFunc(func(w io.Writer) error {
				_, err := w.Write(attach.Data)
				return err
			}),
		)

	}

	return conn.clientSmtp.DialAndSend(m)

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
