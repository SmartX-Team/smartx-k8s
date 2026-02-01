#!/usr/bin/env bash
# Copyright (c) 2025-2026 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
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
    '0x10de')
        # Use host drivers if possible
        if (
            ls /lib/modules/$(uname -r)/kernel/nvidia-*/nvidia.ko >/dev/null 2>/dev/null || \
            ls /lib/modules/$(uname -r)/updates/dkms/nvidia.ko.zst >/dev/null 2>/dev/null
        ) && modprobe nvidia ; then
            echo 'Use host drivers...'
            modprobe nvidia_drm
            modprobe nvidia_modeset
            modprobe nvidia_uvm
            driver='nvidia'
        else
            driver='nouveau'
        fi
        ;; # nvidia
    '0x8086') driver='i915' ;;    # intel
    *)
        echo "WARN: Unsupported GPU device: ${pci_id}"
        exec true
        ;;
    esac
fi

# Skip if another driver is running
if [ "x${driver}" != 'xvfio-pci' ] && [ -e "/sys/bus/pci/devices/${pci_id}/driver" ]; then
        last_driver="$(basename "$(realpath "/sys/bus/pci/devices/${pci_id}/driver")")"
        case "${last_driver}" in
        'i915' | 'nouveau' | 'nvidia')
            echo "WARN: Using the current GPU driver: ${pci_id} -> ${last_driver}"
            exec true
            ;;
        esac
    fi
fi

# Build driver
# if [ -d "/lib/modules/$(uname -r)" ]; then
#     if ! dkms autoinstall; then
#         driver='nouveau' # fallback
#     fi
# fi

# Unload old driver
"$(dirname "$0")/pci-unload.sh" "${dev}"

# Load new driver
if [ ! -d "/sys/bus/pci/drivers/${driver}" ]; then
    echo "DEBUG: Load driver: ${driver}"
    until modprobe "${driver}"; do
        echo "WARN: Still loading driver: ${driver}"
        sleep 1
    done
    echo "INFO: Loaded driver: ${driver}"

    # Some GPU drivers (e.g. nouveau) need some time to finish init
    if [ "${driver}" == 'nouveau' ]; then
        sleep 2
    fi
fi

# Bind device
echo "DEBUG: Bind PCI device: ${pci_id} -> ${driver}"
if [ ! -e "/sys/bus/pci/drivers/${driver}/${pci_id}" ]; then
    echo "${driver}" >"${dev}/driver_override"
    if ! echo "${pci_id}" >"/sys/bus/pci/drivers/${driver}/bind"; then
        echo "WARN: Cannot bind to PCI device: ${pci_id} -> ${driver}"
        echo 1 >"${dev}/enable" 2>/dev/null || true
        exec true
    fi
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
    echo "INFO: Bound PCI device: ${pci_id} -> ${driver}"

# Load vfio-pci to the devices in the same IOMMU group
else
    echo "INFO: Bound PCI device: ${pci_id} -> ${driver}"

    # Do not reexec
    propagate="$3"
    if [ "x${propagate}" == 'xfalse' ]; then
        exec true
    fi

    for neighbor_dev in $(
        find -L "${dev}/iommu_group/devices" -mindepth 1 -maxdepth 1 -type d |
            sort -V
    ); do
        # Skip if the same device
        if [ "x$(realpath "${neighbor_dev}")" == "x$(realpath "${dev}")" ]; then
            continue
        fi

        "$0" "$(realpath "${neighbor_dev}")" "${driver}" "false"
    done
fi
