#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Greeting Screen
# Remove PCI devices in an IOMMU group

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Check user
if [ "x$(id -u)" != 'x0' ]; then
    echo 'ERROR: Permission denied (root-only)'
    exec false
fi

port="$1"
if [ "x${port}" == 'x' ]; then
    exec true
fi

PCI_ID_PATTERN='^[0-9]{4}(:[0-9]{2}){2}\.[0-9]$'

for pci_id in $(ls "${port}" | grep -Po "${PCI_ID_PATTERN}"); do
    "$(dirname "$0")/pci-remove.sh" "${port}/${pci_id}"
done
