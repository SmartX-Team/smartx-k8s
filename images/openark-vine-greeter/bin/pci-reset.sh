#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Greeting Screen
# Reset Primary PCI device

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

pci_id="$(basename "${dev}")"
port="$(dirname "$(readlink -e "${dev}")")"
port_id="$(basename "${port}")"

# Skip if integrated
case "$(cat "${dev}/vendor")" in
'0x8086')
    echo "INFO: Integrated device; skipping resetting: ${pci_id}"
    exec true
    ;; # intel
esac

# Remove the devices
"$(dirname "$0")/pci-remove-group.sh" "${port}"

echo "DEBUG: Patch PCI device: ${pci_id}"
bc="$(setpci -s "${port_id}" 'BRIDGE_CONTROL')"
bc_reset="$(printf "%04x" "$(("0x${bc}" | 0x40))")" # Secondary bus reset

# Apply patched Bridge Control
setpci -s "${port_id}" "BRIDGE_CONTROL=${bc_reset}"
sleep 0.01
setpci -s "${port_id}" "BRIDGE_CONTROL=${bc}"
sleep 0.5
echo "INFO: Patched PCI device: ${pci_id}"

# Remove the port
echo "DEBUG: Reset port: ${port_id}"
echo 1 >"${port}/reset"

# Rescan the PCI bus
echo 'DEBUG: Rescan the PCI bus'
echo 1 >"/sys/bus/pci/rescan"

# Check the reloaded device
if [ ! -e "${dev}" ]; then
    echo "ERROR: Failed to reload PCI device: ${pci_id}"
    exec false
fi
echo "INFO: Reset PCI device: ${pci_id}"
