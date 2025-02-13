package model

import "time"

type Type string

const (
	TypePec Type = "pec"
)

type Credentials struct {
	UUID       string    `json:"uuid" gorm:"primaryKey;column:uuid"`
	Type       string    `json:"type" gorm:"size:10;column:type"`
	LastUpdate time.Time `json:"last_update" gorm:"type:DateTime;column:last_update"`
	CreatedAt  time.Time `json:"created_at" gorm:"column:created_at;autoCreateTime"`
	UpdateAt   time.Time `json:"updated_at" gorm:"column:updated_at;autoUpdateTime"`
}

func (c *Credentials) TableName() string {
	return "credentials"
}
