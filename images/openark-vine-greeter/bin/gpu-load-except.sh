#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Greeting Screen
# Load all the other GPU devices

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Check user
if [ "x$(id -u)" != 'x0' ]; then
    echo 'ERROR: Permission denied (root-only)'
    exec false
fi

primary_dev="$1"
if [ "x${primary_dev}" == 'x' ]; then
    exec true
fi

driver="$2"
if [ "x${driver}" == 'x' ]; then
    exec true
fi

for dev in $(
    find -L /sys/bus/pci/devices \
        -maxdepth 2 -mindepth 2 \
        -name 'drm' -type d -exec dirname {} \;
); do
    if [ "x${dev}" == "x${primary_dev}" ]; then
        continue
    fi

    "$(dirname "$0")/pci-load.sh" "${dev}" "${driver}"
done
