package model

import "time"

type Type string

const (
	TypePec Type = "pec"
)

type Credentials struct {
	UUID       string    `gorm:"primaryKey;column:uuid"`
	Type       Type      `gorm:"size:10;column:type"`
	LastUpdate time.Time `gorm:"type:DateTime;column:last_update"`
	CreatedAt  time.Time `gorm:"column:created_at;autoCreateTime"`
	UpdateAt   time.Time `gorm:"column:updated_at;autoUpdateTime"`
}

func (c *Credentials) TableName() string {
	return "credentials"
}
