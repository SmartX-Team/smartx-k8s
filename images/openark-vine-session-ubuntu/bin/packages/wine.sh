#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Install cutting-edge Wine

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

WINE_REPO="https://github.com/Kron4ek/Wine-Builds/releases/download"
WINE_VERSION_API="https://api.github.com/repos/Kron4ek/Wine-Builds/releases/latest"

apt-mark hold wine

# Get the latest version
## ARCH
case "$(uname -m)" in
'i386') WINE_ARCH='x86' ;;
'x86_64') WINE_ARCH='amd64' ;;
*)
    echo "Unsupported WINE Architechure: '$(uname -m)'"
    exit 1
    ;;
esac

WINE_VERSION="$(
    curl -s "${WINE_VERSION_API}" |
        grep -Po '"tag_name": +"\K[0-9.]+'
)"

# Download
WINE_OBJ_NAME="wine-${WINE_VERSION}-staging-tkg-${WINE_ARCH}"
WINE_OBJ_FILENAME="${WINE_OBJ_NAME}.tar.xz"
WINE_OBJ_FILE="${WINE_OBJ_FILENAME}"
WINE_SRC="/opt/${WINE_OBJ_NAME}"
curl -Lo "${WINE_OBJ_FILE}" "${WINE_REPO}/${WINE_VERSION}/${WINE_OBJ_FILENAME}"

# Decompress the downloaded file
tar -x -C "$(dirname "${WINE_SRC}")" -f "${WINE_OBJ_FILE}"
tar -cf - -C "${WINE_SRC}" . | tar -xf - -C '/usr'

# Cleanup
rm -rf "${WINE_OBJ_FILE}" "${WINE_SRC}"
