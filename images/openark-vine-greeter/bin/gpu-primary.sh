#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Greeting Screen
# Get Primary GPU device

# Prehibit errors
set -e -o pipefail

# Check user
if [ "x$(id -u)" != 'x0' ]; then
    echo 'ERROR: Permission denied (root-only)'
    exec false
fi

# Unload all graphics modules
modules=(
    'i915'
    'nouveau'
    'nvidia_drm'
    'nvidia_modeset'
    'nvidia_uvm'
    'nvidia'
)
for module in ${modules[@]}; do
    while [ -d "/sys/module/${module}" ]; do
        rmmod "${module}" || sleep 0.1
    done
done

primary_dev=''
for dev in $(
    find -L /sys/bus/pci/devices \
        -maxdepth 2 -mindepth 2 -name 'boot_vga'
); do
    if [ "x$(cat "${dev}")" == 'x1' ]; then
        primary_dev="$(dirname "${dev}")"
        break
    fi
done
exec echo "${primary_dev}"
