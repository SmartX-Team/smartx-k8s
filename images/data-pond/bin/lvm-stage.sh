#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Stage LVM volumes

# Prehibit errors
set -e -o pipefail

# Parse inputs
inputs="$(cat | jq)"

# Stage bindings
volume_id="$(echo "${inputs}" | jq -r '.volume_id')"
declare -a bindings=()
for index in $(seq 0 "$(("$(echo "${inputs}" | jq -r '.bindings[].device_id' | wc -l)" - 1))"); do
    bindings+=(
        $(
            echo "${inputs}" |
                jq -r ".bindings[${index}]" |
                "$(dirname "$0")/$(echo "${inputs}" | jq -r ".bindings[${index}].source")-target-stage.sh" \
                    "${volume_id}"
        )
    )
done

# Aggregate bindings
device_path="/dev/${volume_id}/${volume_id}"
if ! vgs -q "${volume_id}" >/dev/null 2>/dev/null; then
    vgcreate \
        --addtag "data-pond.csi.ulagbulag.io/volume_id=${volume_id}" \
        "${volume_id}" ${bindings[@]}
fi
if ! lvs -q "${volume_id}/${volume_id}" >/dev/null 2>/dev/null; then
    lvcreate \
        --addtag "data-pond.csi.ulagbulag.io/volume_id=${volume_id}" \
        -l '100%FREE' \
        -n "${volume_id}" \
        --zero n \
        "${volume_id}"
fi

# Format the volume
fs_type="$(echo "${inputs}" | jq -r '.options.fs_type')"
old_fs="$(blkid -o value -s TYPE "${device_path}" || true)"
if [ "x${old_fs}" == 'x' ]; then
    yes | mkfs -t "${fs_type}" "${device_path}"
elif [ "${old_fs}" != "${fs_type}" ]; then
    echo "Invalid filesystem: expected ${fs_type}, but given ${old_fs}" >&2
    exec false
fi

# Mount the volume
target_path="$(echo "${inputs}" | jq -r '.staging_target_path')"
mkdir -p "${target_path}"
exec mount "${device_path}" "${target_path}"
