#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Discover all available NVMe volumes

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# LVM VGs
for vg in $(vgs --noheadings -o vg_name | awk '{$1=$1};1'); do
    echo '{
        "id": "'"${vg}"'",
        "layer": 1,
        "source": 1,
        "capacity": '"$(vgs --noheadings --units b --nosuffix -o vg_size "${vg}" | awk '{$1=$1};1')"',
        "group": true
}' | jq -c
done
