#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Unpublish a volume from NVMe-oF

# Prehibit errors
set -e -o pipefail

# Parse inputs
volume_id="$1"
device_path="$2"

SUBSYSTEM_HOME="/sys/kernel/config/nvmet/subsystems/${NVME_SUBSYSTEM_NQN}:${volume_id}"
if [ ! -d "${SUBSYSTEM_HOME}" ]; then
    exec true
fi

# Detach ports
for port in $(
    find '/sys/kernel/config/nvmet/ports/' -mindepth 1 -maxdepth 1 -type d |
        sort
); do
    if [ -L "${port}/subsystems/${NVME_SUBSYSTEM_NQN}:${volume_id}" ]; then
        rm "${port}/subsystems/${NVME_SUBSYSTEM_NQN}:${volume_id}"
    fi
done

# Remove volumes from the subsystem
for namespace in $(
    find "${SUBSYSTEM_HOME}/namespaces/" -mindepth 1 -maxdepth 1 -type d |
        sort
); do
    echo 0 >"${namespace}/enable"
    rmdir "${namespace}"
done

# Remove the subsystem
exec rmdir "${SUBSYSTEM_HOME}"
