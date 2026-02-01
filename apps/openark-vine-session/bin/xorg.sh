#!/usr/bin/env bash
# Copyright (c) 2025-2026 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Remote all old sockets
find /tmp/.ICE-unix -maxdepth 1 -type s,p -exec rm '{}' \;
find /tmp/.X11-unix -maxdepth 1 -type s,p -exec rm '{}' \;

# Patch xorg.conf
# cp /opt/X11/xorg.conf.d/desktop/* /etc/X11/xorg.conf.d/

# Configure X
if [ ! -f /etc/X11/xorg.conf ]; then
    X -configure
    ln -sf "${HOME}/xorg.conf.new" /etc/X11/xorg.conf
fi

# Check for input devices to be bound
if ! find /dev/input -maxdepth 1 -type c; then
    exec false
fi

# Use proprietary drivers
if [ -d "/sys/bus/pci/drivers/nvidia" ]; then
    sed -i 's/^\(\t\+Driver \+\)"modesetting"$/\1"nvidia"/g' /etc/X11/xorg.conf
    sed -i 's/^\(\t\+\)BusID\( \+\)"PCI:[0-9]\+:[0-9]\+:[0-9]\+"$/\1VendorName\2"NVIDIA Corporation"/g' /etc/X11/xorg.conf
fi

# Show about the xorg.conf
cat /etc/X11/xorg.conf

# Detect the volumes
cat /proc/mounts

# Detect the GPU drivers
echo '* Detecting GPU drivers...'
if [ -f '/usr/bin/nvidia-smi' ]; then
    nvidia-smi
fi

# Open Xorg session
echo '* Starting Xorg...'
exec Xorg "${DISPLAY}"
