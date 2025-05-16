#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Cleanup NVMe connections

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Mount ConfigFS
if ! cat /proc/mounts | grep -Posq '^configfs +/sys/kernel/config +configfs'; then
    mount configfs -t configfs /sys/kernel/config
fi

# Cleanup Connections
nvme disconnect-all

# Set NVMe NQN
cat /sys/class/dmi/id/product_uuid |
    sha256sum |
    sed -E 's/^(.{8})(.{4})(.{4})(.{4})(.{12}).*$/\1-\2-4\3-8\4-\5/' >/etc/nvme/hostid
echo "nqn.2014-08.org.nvmexpress:uuid:$(cat /etc/nvme/hostid)" >/etc/nvme/hostnqn
