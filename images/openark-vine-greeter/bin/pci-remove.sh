#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Greeting Screen
# Remove a PCI device

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

# Unload the device
"$(dirname "$0")/pci-unload.sh" "${dev}"

# Remove the device
echo "DEBUG: Remove device: ${pci_id}"
echo 1 >"${dev}/remove"
echo "INFO: Removed device: ${pci_id}"
