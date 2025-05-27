#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Publish a volume into NVMe-oF

# Prehibit errors
set -e -o pipefail

# Parse inputs
volume_id="$1"
device_path="$2"

SUBSYSTEM_HOME="/sys/kernel/config/nvmet/subsystems/${NVME_SUBSYSTEM_NQN}:${volume_id}"

# Create a subsystem
if [ ! -d "${SUBSYSTEM_HOME}" ]; then
    mkdir -p "${SUBSYSTEM_HOME}"
    cd "${SUBSYSTEM_HOME}"
    (
        echo 1 >./attr_allow_any_host
    )

    # Add a volume into the subsystem
    namespace_id="$(("$(ls 'namespaces/' | wc -l)" + 1))"
    mkdir "./namespaces/${namespace_id}"
    pushd "./namespaces/${namespace_id}" >/dev/null
    if [ "$(cat ./enable)" == '0' ]; then
        echo -n "${device_path}" >./device_path
        echo 1 >./enable
    fi
    popd >/dev/null
fi

# Attach ports
for port in $(
    find '/sys/kernel/config/nvmet/ports/' -mindepth 1 -maxdepth 1 -type d |
        sort
); do
    if [ ! -L "${port}/subsystems/${NVME_SUBSYSTEM_NQN}:${volume_id}" ]; then
        ln -s "${SUBSYSTEM_HOME}" "${port}/subsystems/${NVME_SUBSYSTEM_NQN}:${volume_id}"
    fi
done
