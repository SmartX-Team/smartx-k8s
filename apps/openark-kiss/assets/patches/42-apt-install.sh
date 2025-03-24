#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# APT Packages Configuration
# Install Dependencies

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

apt-get install -y \
    build-essential \
    containerd \
    curl \
    git \
    haveged \
    iw \
    network-manager \
    nfs-common \
    pciutils \
    ubuntu-server \
    vim \
    wget \
    wireless-tools
