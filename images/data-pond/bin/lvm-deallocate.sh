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

# Deallocate a volume
exec lvremove -f \
    "$(echo "${inputs}" | jq -r '.device_id')/$(echo "${inputs}" | jq -r '.volume_id')"
