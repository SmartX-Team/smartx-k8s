#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Greeting Screen
# Unload a PCI device

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
port="$(dirname "$(readlink -e "${dev}")")"

if [ -e "${dev}/driver/module" ]; then
    driver="$(basename $(readlink -e "${dev}/driver/module"))"

    echo "DEBUG: Unbind PCI device: ${pci_id} <- ${driver}"
    echo "${pci_id}" >"${dev}/driver/unbind"
    if [ "x$(cat "${dev}/enable")" == 'x1' ]; then
        echo 0 >"${dev}/enable"
    fi
    echo "INFO: Unbinded PCI device: ${pci_id} <- ${driver}"

    PCI_ID_PATTERN='^[0-9]{4}(:[0-9]{2}){2}\.[0-9]$'
    if [ "x${driver}" != 'xvfio-pci' ] && ! ls "/sys/bus/pci/drivers/${driver}" |
        grep -Posq "${PCI_ID_PATTERN}"; then
        echo "DEBUG: Unload driver: ${driver}"
        if rmmod "${driver}"; then
            echo "INFO: Unloaded driver: ${driver}"
        else
            echo "WARN: Failed to unload driver: ${driver}; ignoring"
        fi
    fi
fi
