#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Greeting Screen
# Load all USB devices

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Check user
if [ "x$(id -u)" != 'x0' ]; then
    echo 'ERROR: Permission denied (root-only)'
    exec false
fi

driver="$1"
if [ "x${driver}" == 'x' ]; then
    exec true
fi

for dev in $(
    find -L /sys/bus/pci/devices \
        -maxdepth 2 -mindepth 2 \
        -name class -type f -exec dirname {} \; |
        uniq
); do
    # Find all USB controllers
    if ! cat "${dev}/class" | grep -Posq '^0x0c03[0-9a-f]{2}$'; then
        continue
    fi

    # Load driver
    "$(dirname "$0")/pci-load.sh" "${dev}" "${driver}"
done
