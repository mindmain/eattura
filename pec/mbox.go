package pec

import (
	"context"
	"fmt"
	"log"

	"github.com/emersion/go-imap"
)

const domainSdiEmail = "@pec.fatturapa.it"

type pecMailBox struct {
	conn *connection
	name string
}

func (mbox *pecMailBox) ReadAll(ctx context.Context) (<-chan *Message, error) {
	return mbox.Read(ctx, nil)
}
func (mbox *pecMailBox) Read(ctx context.Context, request *SearchMessage) (<-chan *Message, error) {

	_, err := mbox.conn.clientImap.Select(mbox.name, true)

	if err != nil {
		return nil, err
	}

	criteria := imap.NewSearchCriteria()

	if request.Direction == "" {
		request.Direction = DirectionInbound
	}

	if request.Direction == DirectionInbound {
		criteria.Header.Add("From", domainSdiEmail)
	} else if request.Direction == DirectionOutbound {
		criteria.Header.Add("To", domainSdiEmail)
	} else {
		return nil, fmt.Errorf("direction not valid")
	}

	if request != nil {
		if !request.FromAt.IsZero() {
			criteria.Since = request.FromAt
		}

		if !request.ToAt.IsZero() {
			criteria.Before = request.ToAt
		}

	}

	msgs, err := mbox.conn.clientImap.Search(criteria)

	if err != nil {
		return nil, err
	}

	if request != nil {
		if request.Tail > 0 {
			if len(msgs) > request.Tail {
				msgs = msgs[:request.Tail]
			}
		}
	}

	return mbox.searchImap(msgs)

}

func (mbox *pecMailBox) Name() string {
	return mbox.name
}

func (mbox *pecMailBox) searchImap(msgs []uint32) (<-chan *Message, error) {

	var messages = make(chan *Message, 10)

	if len(msgs) == 0 {
		return nil, nil
	}

	go func() {

		for _, seqNum := range msgs {

			imapMessages := make(chan *imap.Message, 1)
			done := make(chan error, 1)
			go func() {
				done <- mbox.conn.clientImap.Fetch(&imap.SeqSet{
					Set: []imap.Seq{{Start: seqNum, Stop: seqNum}},
				}, []imap.FetchItem{
					imap.FetchRFC822,
					imap.FetchEnvelope,
				}, imapMessages)
			}()

			messages <- parseMessage(<-imapMessages, mbox.conn)

			if err := <-done; err != nil {
				log.Println(err)
				break
			}

		}
		close(messages)
	}()

	return messages, nil

}
