package model

import "time"

type Issuer struct {
	UUID string `gorm:"primaryKey;column:uuid"`

	ContactReference string   `gorm:"index;column:contact_uuid"`
	Contact          *Contact `gorm:"foreignKey:ContactReference;references:UUID"`

	CredentialUUID string       `gorm:"index;column:credential_uuid"`
	Credential     *Credentials `gorm:"foreignKey:CredentialUUID;references:UUID"`

	CreatedAt time.Time `gorm:"autoCreateTime;column:created_at"`
	UpdatedAt time.Time `gorm:"autoUpdateTime;column:updated_at"`
}

func (i *Issuer) TableName() string {
	return "issuers"
}
