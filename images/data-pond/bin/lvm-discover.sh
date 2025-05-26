#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Discover all allocated LVM volumes

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# LVM LVs
for vg in $(vgs --noheadings -o vg_name | awk '{$1=$1};1'); do
    tags="$(vgs --noheadings -o tags "${vg}" | tr ',' '\n')"
    if echo "${tags}" | grep -Posq '(^ *|,)data-pond.csi.ulagbulag.io/volume_id=[0-9a-z_-]+(,|$)'; then
        continue
    fi
    for lv in $(lvs --noheadings -o lv_name "${vg}" | grep -Po '^ *\Kpond_pvc-[0-9a-f-]*'); do
        tags="$(lvs --noheadings -o tags "${vg}/${lv}" | tr ',' '\n')"
        if ! echo "${tags}" | grep -Posq '(^ *|,)data-pond.csi.ulagbulag.io/volume_id=[0-9a-z_-]+(,|$)'; then
            continue
        fi
        echo '{
            "volume_id": "'"$(echo "${tags}" | grep -Po 'data-pond.csi.ulagbulag.io/volume_id=\K[a-z0-9_-]+$')"'",
            "device_id": "'"$(echo "${tags}" | grep -Po 'data-pond.csi.ulagbulag.io/device_id=\K[a-z0-9_-]+$')"'",
            "index_bindings": '"$(echo "${tags}" | grep -Po 'data-pond.csi.ulagbulag.io/index_bindings=\K[a-z0-9_-]+$')"',
            "total_bindings": '"$(echo "${tags}" | grep -Po 'data-pond.csi.ulagbulag.io/total_bindings=\K[a-z0-9_-]+$')"',
            "offset": '"$(echo "${tags}" | grep -Po 'data-pond.csi.ulagbulag.io/offset=\K[a-z0-9_-]+$')"',
            "reserved": '"$(echo "${tags}" | grep -Po 'data-pond.csi.ulagbulag.io/reserved=\K[a-z0-9_-]+$')"'
}' | jq -c
    done
done
