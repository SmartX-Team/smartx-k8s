#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Allocate a volume from LVM VG

# Prehibit errors
set -e -o pipefail

# Parse inputs
inputs="$(cat | jq)"

# Parse arguments
device_id="$(echo "${inputs}" | jq -r '.device_id')"
volume_id="$(echo "${inputs}" | jq -r '.volume_id')"
index_bindings="$(echo "${inputs}" | jq -r '.index_bindings')"
total_bindings="$(echo "${inputs}" | jq -r '.total_bindings')"
offset="$(echo "${inputs}" | jq -r '.offset')"
reserved="$(echo "${inputs}" | jq -r '.reserved')"

# Allocate a volume
if ! lvs -q "${device_id}/${volume_id}" >/dev/null 2>/dev/null; then
    lvcreate \
        --addtag "data-pond.csi.ulagbulag.io/device_id=${device_id}" \
        --addtag "data-pond.csi.ulagbulag.io/volume_id=${volume_id}" \
        --addtag "data-pond.csi.ulagbulag.io/index_bindings=${index_bindings}" \
        --addtag "data-pond.csi.ulagbulag.io/total_bindings=${total_bindings}" \
        --addtag "data-pond.csi.ulagbulag.io/offset=${offset}" \
        --addtag "data-pond.csi.ulagbulag.io/reserved=${reserved}" \
        -L "${reserved}B" \
        -n "${volume_id}" \
        --zero n \
        "${device_id}"
fi

# Publish the volume
exec "$(dirname "$0")/$(echo "${inputs}" | jq -r '.source')-source-kernel-publish.sh" \
    "${volume_id}" \
    "/dev/${device_id}/${volume_id}"
