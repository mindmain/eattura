#!/bin/sh
set -eu

if [ "$#" -lt 1 ]; then
  printf 'Missing executable for macOS development signing\n' >&2
  exit 1
fi

identity=${EATTURA_DEV_SIGNING_IDENTITY:?Missing development signing identity}
codesign --force --sign "$identity" --identifier com.mindmain.eattura "$1"
exec "$@"
