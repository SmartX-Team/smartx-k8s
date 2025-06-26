#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Install cutting-edge weston (>=14)

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

WESTON_REPO="https://gitlab.freedesktop.org/wayland/weston.git"
WESTON_VERSION="12.0"

# Download
WESTON_SRC="/opt/weston"
git clone "${WESTON_REPO}" "${WESTON_SRC}" -b "${WESTON_VERSION}"
cd "${WESTON_SRC}"
git submodule update --init --depth=1

# Build
meson build/ --prefix=/usr
ninja -C build/ install

# Cleanup
cd -
rm -rf "${WESTON_SRC}"
