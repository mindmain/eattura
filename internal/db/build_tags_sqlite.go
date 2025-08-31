//go:build sqlite

package db

import "github.com/mindmain/eattura/db/driver/sqlite"

var _sqlite = sqlite.New
