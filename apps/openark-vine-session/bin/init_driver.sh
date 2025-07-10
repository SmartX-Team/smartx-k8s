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
updated='0'
for dev in $(
    find -L /sys/bus/pci/devices \
        -maxdepth 2 -mindepth 2 \
        -name class -type f -exec dirname {} \; |
        uniq
); do
    # Find all USB controllers
    class="$(cat "${dev}/class")"
    if ! echo "${class}" | grep -Posq '^0x0c03[0-9a-f]{2}$'; then
        continue
    fi
    pci_id="$(basename "${dev}")"

    # Unload vfio-pci driver
    if cat "${dev}/driver_override" | grep -Posq '^vfio-pci$'; then
        if [ -d "${dev}/driver" ]; then
            echo "${pci_id}" > "${dev}/driver/unbind"
        fi
        echo '' > "${dev}/driver_override"
        
        # Reload the device
        echo 1 > "${dev}/rescan"
    fi

    # Enable the device
    if [ "x$(cat "${dev}/enable")" == 'x0' ]; then
        echo 1 > "${dev}/enable"
        sleep 0.05
    fi

    # Load new driver
    if echo "${class}" | grep -Posq '^0x0c0330$'; then
        if [ "x$(cat "${dev}/enable")" == 'x1' ] && [ ! -L "${dev}/driver" ]; then
            echo "${pci_id}" > '/sys/bus/pci/drivers/xhci_hcd/bind'
            updated='1'
        fi
    fi
done

# Find the primary GPU device
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

if [ "x${primary_dev}" != 'x' ]; then
    dev="${primary_dev}"
    pci_id="$(basename "${dev}")"

    # Try seleting the builtin GPU driver module
    driver=''
    case "$(cat "${dev}/vendor")" in
    '0x10de')
        echo "INFO: Ignoring NVIDIA GPU driver: ${pci_id}"
        ;; # nvidia
    '0x8086') driver='i915' ;; # intel
    esac

    if [ "x${driver}" != 'x' ]; then
        # Unload vfio-pci driver
        if cat "${dev}/driver_override" | grep -Posq '^vfio-pci$'; then
            if [ -d "${dev}/driver" ]; then
                echo "${pci_id}" > "${dev}/driver/unbind"
            fi
            echo '' > "${dev}/driver_override"
        fi

        # Reload the device
        echo 1 > "${dev}/rescan"

        # Enable the device
        if [ "x$(cat "${dev}/enable")" == 'x0' ]; then
            echo 1 > "${dev}/enable"
            sleep 0.05
        fi

        # Load new driver
        if [ ! -d "/sys/bus/pci/drivers/${driver}" ]; then
            modprobe "${driver}"
            updated='1'
            sleep 1
        fi

        # Bind device
        if [ ! -e "/sys/bus/pci/drivers/${driver}/${pci_id}" ]; then
            echo "${driver}" >"${dev}/driver_override"
            echo "${pci_id}" >"/sys/bus/pci/drivers/${driver}/bind"
            updated='1'
        fi
    fi
fi

# Apply updates
if [ "x${updated}" == 'x1' ]; then
    modprobe 'xhci_pci'
    sleep 2
fi
