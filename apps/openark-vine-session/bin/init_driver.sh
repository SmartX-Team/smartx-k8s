#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Check user
if [ "x$(id -u)" != 'x0' ]; then
    echo 'ERROR: Permission denied (root-only)'
    exec false
fi

# Load XHCI driver
modprobe 'xhci_pci'

# Load all USB drivers
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
    if ! cat "${dev}/driver_override" | grep -Posq '^vfio-pci$'; then
        continue
    fi

    # Unload vfio-pci driver
    pci_id="$(basename "${dev}")"
    if [ -d "${dev}/driver" ]; then
        echo "${pci_id}" > "${dev}/driver/unbind"
    fi
    echo '' > "${dev}/driver_override"
    
    # Reload the device
    echo 1 > "${dev}/rescan"

    # Load new driver
    echo "${pci_id}" > '/sys/bus/pci/drivers/xhci_hcd/bind' || true
done
