package model

import "time"

type Type string

const (
	TypePec Type = "pec"
)

type Credentials struct {
	UUID       string    `json:"uuid" xorm:"pk 'uuid'"`
	Type       string    `json:"type" xorm:"varChar(10) 'type'"`
	LastUpdate time.Time `json:"last_update" xorm:"DATETIME 'last_update'"`
	CreatedAt  time.Time `json:"created_at" xorm:"created 'created_at'"`
	UpdateAt   time.Time `json:"updated_at" xorm:"updated 'updated_at'"`
}

func (c *Credentials) TableName() string {
	return "credentials"
}
