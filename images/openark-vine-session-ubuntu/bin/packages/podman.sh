#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Install cutting-edge podman

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

CRUN_REPO="https://github.com/containers/crun.git"
CRUN_VERSION="1.15"
PODMAN_REPO="https://github.com/containers/podman.git"
PODMAN_VERSION="5.1"

apt-mark hold crun podman

# crun

## Download
CRUN_SRC="/opt/crun"
git clone "${CRUN_REPO}" "${CRUN_SRC}" -b "${CRUN_VERSION}"
cd "${CRUN_SRC}"
git submodule update --init --depth=1

## Build
./autogen.sh
./configure --enable-shared --prefix=/usr
make
make install

# podman

## Download
PODMAN_SRC="/opt/podman"
git clone "${PODMAN_REPO}" "${PODMAN_SRC}" -b "v${PODMAN_VERSION}"
cd "${PODMAN_SRC}"
## Build
make BUILDTAGS='cni seccomp selinux systemd' PREFIX=/usr
make install PREFIX=/usr

# Cleanup
cd -
rm -rf "${CRUN_SRC}" "${PODMAN_SRC}"
