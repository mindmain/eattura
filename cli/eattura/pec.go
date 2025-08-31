package main

import (
	"os"
	"path/filepath"

	"github.com/mindmain/eattura/internal/pec"
	"github.com/mindmain/eattura/internal/secure"
	"github.com/spf13/cobra"
)

var commandUUID string

func PecCommand() *cobra.Command {
	cmd := &cobra.Command{
		Use: "pec",
	}

	cmd.AddCommand(commandMbox())
	cmd.AddCommand(commandSend())
	cmd.PersistentFlags().StringVar(&commandUUID, "uuid", "", "uuid")

	return cmd

}

func commandMbox() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "mbox",
		Short: "mailbox operations",
	}

	cmd.AddCommand(commandListMbox())
	cmd.AddCommand(commandRead())
	return cmd
}

func commandListMbox() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "list",
		Short: "list mailboxes",
		Run: func(cmd *cobra.Command, args []string) {

			ss, err := secure.New()

			if err != nil {
				cmd.Println(err)
				return
			}

			client := pec.NewPEC(ss)

			conn, err := client.Connect(cmd.Context(), commandUUID)

			if err != nil {
				cmd.Println(err)
				return
			}
			defer conn.Close()

			boxes, err := conn.Boxes(cmd.Context())

			if err != nil {
				cmd.Println(err)
				return
			}

			for _, box := range boxes {
				cmd.Println(box.Name())
			}

		},
	}

	return cmd
}

func commandRead() *cobra.Command {

	var all bool
	var box string
	var tail int
	cmd := &cobra.Command{
		Use:   "read",
		Short: "read messages",
		Run: func(cmd *cobra.Command, args []string) {

			ss, err := secure.New()

			if err != nil {
				cmd.Println(err)
				return
			}

			client := pec.NewPEC(ss)

			conn, err := client.Connect(cmd.Context(), commandUUID)

			if err != nil {
				cmd.Println(err)
				return
			}
			defer conn.Close()

			mbox, err := conn.Box(cmd.Context(), box)

			if err != nil {
				cmd.Println(err)
				return
			}

			if all {
				messages, err := mbox.ReadAll(cmd.Context())

				if err != nil {
					cmd.Println(err)
					return
				}

				for msg := range messages {
					cmd.Println(msg)
				}
			} else {
				messages, err := mbox.Read(cmd.Context(), &pec.SearchMessage{
					Tail: tail,
				})

				if err != nil {
					cmd.Println(err)
					return
				}

				for msg := range messages {
					cmd.Println(msg)
				}
			}

		},
	}
	cmd.Flags().StringVar(&box, "box", "INBOX", "mailbox")
	cmd.Flags().BoolVar(&all, "all", false, "read all messages")
	cmd.Flags().IntVar(&tail, "tail", 5, "read last n messages")
	return cmd

}

func commandSend() *cobra.Command {

	var attachments []string
	var subject string
	cmd := &cobra.Command{
		Use:   "send",
		Short: "send message",
		Run: func(cmd *cobra.Command, args []string) {

			ss, err := secure.New()

			if err != nil {
				cmd.Println(err)
				return
			}

			client := pec.NewPEC(ss)

			conn, err := client.Connect(cmd.Context(), commandUUID)

			if err != nil {
				cmd.Println(err)
				return
			}
			defer conn.Close()

			msg := &pec.RequestSend{
				Subject: subject,
			}

			for _, attach := range attachments {
				data, err := os.ReadFile(attach)

				if err != nil {
					cmd.Println(err)
					return
				}

				msg.AddAttachment(filepath.Base(attach), data)
			}

			if err := conn.Send(cmd.Context(), msg); err != nil {
				cmd.Println(err)
				return
			} else {
				cmd.Println("message sent")
			}
		},
	}

	cmd.Flags().StringVarP(&subject, "subject", "s", "", "subject")
	cmd.Flags().StringSliceVarP(&attachments, "attachments", "a", nil, "attachments")

	return cmd
}
