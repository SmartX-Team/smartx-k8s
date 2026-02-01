#!/usr/bin/env bash
# Copyright (c) 2026 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Install cutting-edge Steam

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

WINE_REPO="https://github.com/Kron4ek/Wine-Builds/releases/download"
WINE_VERSION_API="https://api.github.com/repos/Kron4ek/Wine-Builds/releases/latest"

# Download
STEAM_SRC="/tmp/steam-launcher_latest_all.deb"
curl -Lo "${STEAM_SRC}" "https://repo.steampowered.com/steam/archive/stable/steam-launcher_latest_all.deb"

# Install
apt-get install -y "${STEAM_SRC}"

# Cleanup
rm "${STEAM_SRC}"
