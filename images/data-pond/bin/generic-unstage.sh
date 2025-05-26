#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Unstage volumes

# Prehibit errors
set -e -o pipefail

# Parse inputs
inputs="$(cat | jq)"
echo "$inputs" >&2

# Unmount the volume
target_path="$(echo "${inputs}" | jq -r '.target_path')"
if [ -d "${target_path}" ]; then
    umount "${target_path}"
    rmdir "${target_path}"
fi

# Disaggregate devices
# FIXME: To be implemented! (LVM)
volume_id="$(echo "${inputs}" | jq -r '.volume_id')"
if [ -d "${volume_id}" ]; then
    lvremove -f "${volume_id}/${volume_id}"
    vgremove -f "${volume_id}"
fi

# Unstage devices
# FIXME: To be implemented! (NVMe, NVMe-oF)
exec true
