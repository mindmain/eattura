package model

import "time"

type Type string

const (
	TypePec Type = "pec"
)

type Credentials struct {
	UUID       string    `json:"uuid" xorm:"pk"`
	Type       string    `json:"type" xorm:"varChar(10)"`
	LastUpdate time.Time `json:"last_update" xorm:"DATETIME"`
	CreatedAt  time.Time `json:"created_at" xorm:"created"`
	UpdateAt   time.Time `json:"updated_at" xorm:"updated"`
}

func (c *Credentials) TableName() string {
	return "credentials"
}
