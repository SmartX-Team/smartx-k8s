#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Deallocate a volume from LVM VG

# Prehibit errors
set -e -o pipefail

# Parse inputs
inputs="$(cat | jq)"

# Parse arguments
device_id="$(echo "${inputs}" | jq -r '.device_id')"
volume_id="$(echo "${inputs}" | jq -r '.volume_id')"

# Unpublish the volume
"$(dirname "$0")/$(echo "${inputs}" | jq -r '.source')-source-kernel-unpublish.sh" \
    "${volume_id}" \
    "/dev/${device_id}/${volume_id}"

# Deallocate a volume
if [ -L "/dev/${device_id}/${volume_id}" ]; then
    pvremove -f "/dev/${device_id}/${volume_id}" || true
    lvremove -f "${device_id}/${volume_id}"
fi
