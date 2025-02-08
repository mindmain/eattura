package pec

import (
	"context"
	"log"

	"github.com/emersion/go-imap"
)

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
	criteria.Header.Add("From", "@pec.fatturapa.it")

	if request != nil {
		if !request.From.IsZero() {
			criteria.Since = request.From
		}

		if !request.To.IsZero() {
			criteria.Before = request.To
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
					//	imap.FetchBody,
					//	imap.FetchBodyStructure,
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
