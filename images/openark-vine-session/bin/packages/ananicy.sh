#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Install cutting-edge ananicy

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

ANANICY_REPO="https://github.com/kuche1/minq-ananicy.git"
ANANICY_VERSION="master"

# Download
ANANICY_SRC="/opt/ananicy"
git clone "${ANANICY_REPO}" "${ANANICY_SRC}" -b "${ANANICY_VERSION}"
cd "${ANANICY_SRC}"
git submodule update --init --depth=1

# Build
./package.sh debian
apt-get install -y ./ananicy-*.deb

# Cleanup
cd -
rm -rf "${ANANICY_SRC}"
