#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Greeting Screen
# Load a PCI device

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Check user
if [ "x$(id -u)" != 'x0' ]; then
    echo 'ERROR: Permission denied (root-only)'
    exec false
fi

dev="$1"
if [ "x${dev}" == 'x' ]; then
    exec true
fi

pci_id="$(basename ${dev})"

# Auto-detect GPU driver module
driver="$2"
if [ "x${driver}" == 'x' ]; then
    exec true
elif [ "x${driver}" == 'xgpu' ]; then
    case "$(cat "${dev}/vendor")" in
    '0x10de') driver='nouveau' ;; # nvidia
    '0x8086') driver='i915' ;;    # intel
    '*')
        echo "WARN: Unsupported PCI device: ${pci_id}"
        exec true
        ;;
    esac
fi

# Unload driver
if [ -e "${dev}/driver" ] && [ -f "${dev}/driver_override" ]; then
    if [ "x$(cat "${dev}/driver_override")" != "x${driver}" ]; then
        "$(dirname "$0")/pci-unload.sh" "${dev}"
        sleep 0.2
    fi
fi

if [ ! -d "/sys/bus/pci/drivers/${driver}" ]; then
    echo "DEBUG: Load driver: ${driver}"
    modprobe "${driver}"
    echo "INFO: Loaded driver: ${driver}"
fi

echo "DEBUG: Bind PCI device: ${pci_id} -> ${driver}"
if [ ! -e "/sys/bus/pci/drivers/${driver}/${pci_id}" ]; then
    echo "${driver}" >"${dev}/driver_override"
    echo "${pci_id}" >"/sys/bus/pci/drivers/${driver}/bind"
fi

# Wait until the VGA card is loaded
if [ "x${driver}" != 'xvfio-pci' ]; then
    while :; do
        attr="$(find "${dev}/drm" -mindepth 1 -maxdepth 1 -name "card*" -type d || true)"
        if [ "x${attr}" != 'x' ]; then
            card="/dev/dri/$(basename "${attr}")"
            break
        fi
        sleep 0.1
    done
    while :; do
        attr="$(find "${dev}/drm" -mindepth 1 -maxdepth 1 -name "renderD*" -type d || true)"
        if [ "x${attr}" != 'x' ]; then
            renderer="/dev/dri/$(basename "${attr}")"
            break
        fi
        sleep 0.1
    done
    until [ -c "${card}" ] && [ -c "${renderer}" ]; do
        sleep 0.1
    done
    echo "INFO: Binded PCI device: ${pci_id} -> ${driver}"

# Load vfio-pci to the devices in the same IOMMU group
else
    # Do not reexec
    propagate="$3"
    if [ "x${propagate}" == 'xfalse' ]; then
        exec true
    fi

    for neighbor_dev in $(
        find -L "${dev}/iommu_group/devices" -mindepth 1 -maxdepth 1 -type d |
            sort -V
    ); do
        if [ "x$(realpath "${neighbor_dev}")" == "x$(realpath "${dev}")" ]; then
            continue
        fi
        "$0" "${neighbor_dev}" "${driver}" "false"
    done
fi
