#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
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

# Open Xorg session
exec Xorg "${DISPLAY}"
