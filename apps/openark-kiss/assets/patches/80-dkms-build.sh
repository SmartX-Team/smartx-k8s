#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Kernel Configuration
# Build DKMS

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

if which dkms >/dev/null 2>/dev/null; then
    SRC_KERNEL_VERSION="$(ls '/lib/modules/' | sort -V | tail -n1)"
    dkms autoinstall -k "${SRC_KERNEL_VERSION}"
fi
