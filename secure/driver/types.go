package driver

type Credentials struct {
	ImapHost   string `json:"imap_host"`
	ImapPort   int    `json:"imap_port"`
	ImapInbox  string `json:"imap_inbox"`
	ImapOutbox string `json:"imap_outbox"`
	ImapTls    bool   `json:"imap_tls"`

	SmtpHost string `json:"smtp_host"`
	SmtpPort int    `json:"smtp_port"`

	PecSdi string `json:"pec_sdi"`

	Username string `json:"username"`
	Password string `json:"password"`
}

type CredentialsBook map[string]*Credentials

// Make an secure credentials for pec and other sensibles data
type Storage interface {

	// Create a new secure credentials
	Create(key string, creds *Credentials) error

	// Get the secure credentials from the file system
	// if not exists return an error
	// if the key is not correct return an error
	Get(key string) (*Credentials, error)

	// Remove the secure credentials from the file system
	Delete(key string) error
}
