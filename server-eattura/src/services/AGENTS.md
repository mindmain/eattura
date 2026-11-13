# Server services

`pec_service.rs` handles SMTP and IMAP. Keep mailbox credentials transient, omit them from logs, and use `BODY.PEEK[]` when polling.
