#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Stage NVMe devices

# Prehibit errors
set -e -o pipefail

# Parse inputs
inputs="$(cat | jq)"
volume_id="$1"

# Parse arguments
device_id="$(echo "${inputs}" | jq -r '.device_id')"
layer="$(echo "${inputs}" | jq -r '.layer')"
# FIXME: To be implemented! (.device.addr => .addr DYNAMIC PROVISIONING ON NODE RUNTIME)
addr="$(echo "${inputs}" | jq -r '.device.addr')"
addr="${POD_IP}"

# Use local devices if possible
if [ "x${addr}" == "x${POD_IP}" ]; then
    case "${layer}" in
    'lvm')
        exec echo "/dev/${device_id}/${volume_id}"
        ;;
    esac
fi

# Connect to the target pond
nvme connect -t tcp -a "${addr}" -n "${NVME_SUBSYSTEM_NQN}:${volume_id}" -s "${NVME_FABRIC_TCP_PORT}"

# Collect available devices
for class in $(
    nvme list-subsys -o json |
        jq -r ".[].Subsystems[] | select(.NQN == \"${NVME_SUBSYSTEM_NQN}:${volume_id}\") | .Paths[].Name"
); do
    nvme list -o json |
        jq -r ".Devices[].DevicePath | select(. | test(\"^/dev/${class}n[0-9]+\"))"
done
