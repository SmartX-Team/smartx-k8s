#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# User Directory Permissions
mkdir -p \
    "${HOME}/.local/share/containers/storage" \
    '/mnt/public' \
    '/run/dbus' \
    "/run/user/${TARGET_UID}" \
    '/tmp/.ICE-unix' \
    '/tmp/.X11-unix'
chown "${TARGET_UID}" \
    "${HOME}/" \
    "${HOME}/.local" \
    "${HOME}/.local/share" \
    "${HOME}/.local/share/containers" \
    "${HOME}/.local/share/containers/storage" \
    '/mnt/public' \
    '/run/dbus' \
    "/run/user/${TARGET_UID}"
chmod 700 \
    "${HOME}" \
    "${HOME}/.local/share/containers/storage" \
    '/run/dbus' \
    "/run/user/${TARGET_UID}"
chmod 777 \
    '/mnt/public'
chmod 1777 \
    '/tmp/.ICE-unix' \
    '/tmp/.X11-unix'

# Network Optimizations
if [ -d /host-sys/module/mac80211/parameters ]; then
    # Make roaming algorithm more loose
    echo 10 >/host-sys/module/mac80211/parameters/beacon_loss_count
    echo 20 >/host-sys/module/mac80211/parameters/max_probe_tries
    echo 4000 >/host-sys/module/mac80211/parameters/probe_wait_ms
fi

exec true
