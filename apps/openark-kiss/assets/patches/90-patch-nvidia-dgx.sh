#!/usr/bin/env bash
# Copyright (c) 2026 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Apply specialized settings
# Patch NVIDIA DGX devices

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Filter nodes
if ! [ -f '/sys/class/dmi/id/product_family' ] || [ "x$(cat '/sys/class/dmi/id/product_family')" != 'xDGX Spark' ]; then
    exit
fi

# Enable watchdog kernel module
# NOTE: https://forums.developer.nvidia.com/t/dgx-spark-keeps-rebooting-every-20-30-minutes/350692/6
sudo apt remove -y linux-generic "linux-headers-6.8.0-*" linux-headers-generic "linux-image-6.8.0-*" linux-image-generic "linux-modules-6.8.0-*" "linux-modules-extra-6.8.0-*"

# Enable wireless networking
nmcli radio wifi on

# Force reboot once
if ! ip link show dev 'wlP9p1s0'; then
    reboot
fi
