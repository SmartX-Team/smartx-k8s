#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Unload NVMe Kernel Modules

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Cleanup Connections
if [ -d '/sys/module/nvme_fabrics' ]; then
    nvme disconnect-all
fi

# nvme
rmmod nvme-tcp || true
rmmod nvme-fabrics || true
