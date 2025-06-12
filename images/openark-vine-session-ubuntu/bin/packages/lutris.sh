#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Install cutting-edge lutris

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

LUTRIS_VERSION_API="https://api.github.com/repos/lutris/lutris/releases/latest"

# Get the latest version
LUTRIS_DOWNLOAD_URL="$(
    curl -s "${LUTRIS_VERSION_API}" |
        grep -Po '"browser_download_url" *\: *"\K[0-9a-zA-Z/:._-]+'
)"

# Download
LUTRIS_FILE="/tmp/lutris.deb"
curl -sSL -o "${LUTRIS_FILE}" "${LUTRIS_DOWNLOAD_URL}"

# Install
apt-get install -y "${LUTRIS_FILE}"

# Cleanup
rm -rf "${LUTRIS_FILE}"
