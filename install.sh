#!/usr/bin/env sh
set -eu

# Thin wrapper around the cargo-dist generated installer published in GitHub Releases.
#
# Usage:
#   curl -sSfL https://raw.githubusercontent.com/sensiarion/apidrift/main/install.sh | sh
#
# Optional env:
#   APIDRIFT_VERSION=v0.1.4   # default: latest
#   APIDRIFT_INSTALL_DIR=...  # forwarded to cargo-dist installer

VERSION="${APIDRIFT_VERSION:-latest}"
REPO="sensiarion/apidrift"

if [ "$VERSION" = "latest" ]; then
  INSTALLER_URL="https://github.com/${REPO}/releases/latest/download/apidrift-installer.sh"
else
  INSTALLER_URL="https://github.com/${REPO}/releases/download/${VERSION}/apidrift-installer.sh"
fi

echo "[INFO] Downloading installer: ${INSTALLER_URL}" >&2

# The cargo-dist installer supports:
# - APIDRIFT_INSTALL_DIR (install prefix)
# - APIDRIFT_NO_MODIFY_PATH=1 (don't edit shell profiles)
curl -sSfL "$INSTALLER_URL" | sh

