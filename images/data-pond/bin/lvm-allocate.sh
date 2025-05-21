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

# Allocate a volume
exec lvcreate \
    -L "$(echo "${inputs}" | jq -r '.capacity')B" \
    -n "$(echo "${inputs}" | jq -r '.volume.id')" \
    --zero n \
    "$(echo "${inputs}" | jq -r '.device_id')"
