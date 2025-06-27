#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Install cutting-edge nvidia-container-toolkit

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

NVIDIA_CONTAINER_TOOLKIT_REPO="https://github.com/ulagbulag/nvidia-container-toolkit.git"
NVIDIA_CONTAINER_TOOLKIT_VERSION="fix/suppress-proc-mount-error-for-non-root-users"

# Download
NVIDIA_CONTAINER_TOOLKIT_SRC="/opt/nvidia-container-toolkit"
git clone "${NVIDIA_CONTAINER_TOOLKIT_REPO}" "${NVIDIA_CONTAINER_TOOLKIT_SRC}" -b "${NVIDIA_CONTAINER_TOOLKIT_VERSION}"
cd "${NVIDIA_CONTAINER_TOOLKIT_SRC}"
git submodule update --init --depth=1

# Build
PREFIX=/usr/bin make binaries

# Cleanup
cd -
rm -rf "${NVIDIA_CONTAINER_TOOLKIT_SRC}"
