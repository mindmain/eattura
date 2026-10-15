#!/bin/sh
set -eu

if [ "$(uname -s)" = "Darwin" ]; then
  script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
  identity=${EATTURA_DEV_SIGNING_IDENTITY:-Eattura Local Development}
  if security find-identity -v -p codesigning | grep -Fq "\"$identity\""; then
    target=$(rustc -vV | awk '/^host: / { print $2 }')
    case "$target" in
      aarch64-apple-darwin|x86_64-apple-darwin) ;;
      *) printf 'Unsupported macOS target: %s\n' "$target" >&2; exit 1 ;;
    esac
    runner_var="CARGO_TARGET_$(printf '%s' "$target" | tr '[:lower:]-' '[:upper:]_')_RUNNER"
    export EATTURA_DEV_SIGNING_IDENTITY="$identity"
    export "$runner_var=$script_dir/macos-dev-signing-runner.sh"
  else
    printf 'No macOS development signing identity found; starting with the existing ad-hoc signature.\n' >&2
  fi
fi

exec bun run tauri dev "$@"
