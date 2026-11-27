#!/bin/sh
# Eattura desktop installer for macOS (Apple Silicon / Intel) and Linux (x86_64).
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/mindmain/eattura/main/scripts/install.sh | sh
#
# Optional environment variables:
#   EATTURA_VERSION      Specific release tag (e.g. "v0.1.1"); defaults to latest release
#   EATTURA_INSTALL_DIR  Destination folder (default: /Applications on macOS, ~/.local/bin on Linux)
#   EATTURA_REPO         GitHub repository in OWNER/REPO format (default: mindmain/eattura)

set -eu

REPO="${EATTURA_REPO:-mindmain/eattura}"
case "$REPO" in
  *[!A-Za-z0-9_./-]* | "" | */*/*)
    echo "Error: invalid EATTURA_REPO '$REPO' (expected OWNER/REPO)." >&2
    exit 1
    ;;
  */*) ;;
  *)
    echo "Error: invalid EATTURA_REPO '$REPO' (expected OWNER/REPO)." >&2
    exit 1
    ;;
esac

VERSION_TAG="${EATTURA_VERSION:-}"
if [ -n "$VERSION_TAG" ]; then
  case "$VERSION_TAG" in
    v[0-9]*.[0-9]*.[0-9]*) ;;
    [0-9]*.[0-9]*.[0-9]*) VERSION_TAG="v${VERSION_TAG}" ;;
    *)
      echo "Error: invalid EATTURA_VERSION '$VERSION_TAG' (expected vMAJOR.MINOR.PATCH)." >&2
      exit 1
      ;;
  esac
fi

if [ -n "${EATTURA_RELEASE_BASE_URL:-}" ]; then
  BASE_URL="${EATTURA_RELEASE_BASE_URL%/}"
  case "$BASE_URL" in
    https://* | file://*) ;;
    *)
      echo "Error: EATTURA_RELEASE_BASE_URL must start with https:// or file://." >&2
      exit 1
      ;;
  esac
elif [ -n "$VERSION_TAG" ]; then
  BASE_URL="https://github.com/${REPO}/releases/download/${VERSION_TAG}"
else
  BASE_URL="https://github.com/${REPO}/releases/latest/download"
fi

OS_NAME="${EATTURA_TEST_OS:-$(uname -s)}"
ARCH_NAME="${EATTURA_TEST_ARCH:-$(uname -m)}"

case "$OS_NAME" in
  Darwin)
    case "$ARCH_NAME" in
      arm64 | aarch64) TARGET_PREFIX="aarch64-apple-darwin-" ;;
      x86_64 | amd64) TARGET_PREFIX="x86_64-apple-darwin-" ;;
      *)
        echo "Error: unsupported macOS architecture '$ARCH_NAME'." >&2
        exit 1
        ;;
    esac
    ASSET_SUFFIX=".app.tar.gz"
    ;;
  Linux)
    case "$ARCH_NAME" in
      x86_64 | amd64) TARGET_PREFIX="x86_64-unknown-linux-gnu-" ;;
      *)
        echo "Error: unsupported Linux architecture '$ARCH_NAME' (x86_64 required)." >&2
        exit 1
        ;;
    esac
    ASSET_SUFFIX=".AppImage"
    ;;
  *)
    echo "Error: unsupported operating system '$OS_NAME'. Download the Windows installer from https://github.com/${REPO}/releases/latest" >&2
    exit 1
    ;;
esac

for cmd in curl tar awk; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Error: required command '$cmd' is not installed." >&2
    exit 1
  fi
done

compute_sha256() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print tolower($1)}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print tolower($1)}'
  elif command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 "$1" | awk '{print tolower($NF)}'
  else
    echo "Error: no SHA-256 tool found (expected shasum, sha256sum, or openssl)." >&2
    exit 1
  fi
}

TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'eattura-install')"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

echo "==> Fetching Eattura release metadata..."
curl -fsSL "${BASE_URL}/SHA256SUMS" -o "${TMP_DIR}/SHA256SUMS"
curl -fsSL "${BASE_URL}/latest.json" -o "${TMP_DIR}/latest.json"

LATEST_EXPECTED_SHA="$(awk '$2 == "latest.json" { print tolower($1); exit }' "${TMP_DIR}/SHA256SUMS")"
if [ -z "$LATEST_EXPECTED_SHA" ]; then
  echo "Error: latest.json entry missing from SHA256SUMS." >&2
  exit 1
fi

LATEST_ACTUAL_SHA="$(compute_sha256 "${TMP_DIR}/latest.json")"
if [ "$LATEST_ACTUAL_SHA" != "$LATEST_EXPECTED_SHA" ]; then
  echo "Error: SHA-256 mismatch for latest.json (expected $LATEST_EXPECTED_SHA, got $LATEST_ACTUAL_SHA)." >&2
  exit 1
fi

RELEASE_VERSION="$(awk -F'"' '/^[[:space:]]*"version":[[:space:]]*"/ { print $4; exit }' "${TMP_DIR}/latest.json")"

MATCH_LINE="$(awk -v prefix="$TARGET_PREFIX" -v suffix="$ASSET_SUFFIX" '
  index($2, prefix) == 1 && substr($2, length($2) - length(suffix) + 1) == suffix {
    print tolower($1) " " $2
    exit
  }
' "${TMP_DIR}/SHA256SUMS")"

if [ -z "$MATCH_LINE" ]; then
  echo "Error: no release asset matching '${TARGET_PREFIX}*${ASSET_SUFFIX}' found in SHA256SUMS." >&2
  exit 1
fi

EXPECTED_SHA="${MATCH_LINE%% *}"
ASSET_NAME="${MATCH_LINE#* }"

case "$ASSET_NAME" in
  *[!A-Za-z0-9._+-]* | "" | ..*)
    echo "Error: unsafe asset filename '$ASSET_NAME'." >&2
    exit 1
    ;;
esac

echo "==> Downloading ${ASSET_NAME}${RELEASE_VERSION:+ (v${RELEASE_VERSION})}..."
ARCHIVE_PATH="${TMP_DIR}/${ASSET_NAME}"
curl -fL --progress-bar "${BASE_URL}/${ASSET_NAME}" -o "$ARCHIVE_PATH"

echo "==> Verifying SHA-256 checksum..."
ACTUAL_SHA="$(compute_sha256 "$ARCHIVE_PATH")"
if [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
  echo "Error: SHA-256 checksum verification failed for ${ASSET_NAME}!" >&2
  echo "  Expected: ${EXPECTED_SHA}" >&2
  echo "  Actual:   ${ACTUAL_SHA}" >&2
  exit 1
fi

if [ "$OS_NAME" = "Darwin" ]; then
  EXTRACT_DIR="${TMP_DIR}/extract"
  mkdir -p "$EXTRACT_DIR"
  tar -xzf "$ARCHIVE_PATH" -C "$EXTRACT_DIR"

  APP_BUNDLE="$(find "$EXTRACT_DIR" -maxdepth 2 -name "*.app" -type d | head -n 1)"
  if [ -z "$APP_BUNDLE" ]; then
    echo "Error: no .app bundle found inside ${ASSET_NAME}." >&2
    exit 1
  fi

  if [ -n "${EATTURA_INSTALL_DIR:-}" ]; then
    DEST_DIR="$EATTURA_INSTALL_DIR"
    mkdir -p "$DEST_DIR"
  elif [ -w "/Applications" ]; then
    DEST_DIR="/Applications"
  else
    DEST_DIR="${HOME}/Applications"
    mkdir -p "$DEST_DIR"
  fi

  APP_NAME="$(basename "$APP_BUNDLE")"
  DEST_APP="${DEST_DIR}/${APP_NAME}"

  echo "==> Installing ${APP_NAME} to ${DEST_DIR}..."
  rm -rf "$DEST_APP"
  if command -v ditto >/dev/null 2>&1; then
    ditto "$APP_BUNDLE" "$DEST_APP"
  else
    cp -R "$APP_BUNDLE" "$DEST_APP"
  fi

  # Ensure no browser quarantine attributes remain on the installed bundle.
  if command -v xattr >/dev/null 2>&1; then
    xattr -cr "$DEST_APP" 2>/dev/null || true
  fi

  echo ""
  echo "Eattura${RELEASE_VERSION:+ v${RELEASE_VERSION}} installed successfully at: ${DEST_APP}"
  echo "You can now open Eattura from Applications or Spotlight (future updates install in-app)."
else
  DEST_DIR="${EATTURA_INSTALL_DIR:-${HOME}/.local/bin}"
  mkdir -p "$DEST_DIR"
  DEST_BIN="${DEST_DIR}/eattura"
  cp "$ARCHIVE_PATH" "$DEST_BIN"
  chmod 0755 "$DEST_BIN"

  echo ""
  echo "Eattura${RELEASE_VERSION:+ v${RELEASE_VERSION}} installed successfully at: ${DEST_BIN}"
  case ":${PATH:-}:" in
    *":${DEST_DIR}:"*) ;;
    *) echo "Note: add ${DEST_DIR} to your PATH to launch 'eattura' from the terminal." ;;
  esac
fi
