package fs

import "errors"

var ErrorInvalidDriverPath = errors.New("invalid driver path")
var ErrorInvalidDriverType = errors.New("invalid driver type")
var ErrorInvalidRootPath = errors.New("invalid root path")
var ErrorPermissionDenied = errors.New("permission denied, can't read/write in the directory")
