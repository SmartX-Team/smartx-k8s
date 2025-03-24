#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Greeting Screen
# Get Primary GPU device

# Prehibit errors
set -e -o pipefail

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
