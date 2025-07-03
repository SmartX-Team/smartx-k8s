#!/usr/bin/env bash
# Copyright (c) 2023-2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

if [ "x$(stat -c '%u' '/var/lib/containers')" == "x$(id -u)" ]; then
    PREFIX=''
else
    PREFIX='sudo '
fi

function _reset_podman() {
    # Initialize rootless podman
    ${PREFIX} podman system migrate

    # Generate a CDI specification that refers to all NVIDIA devices
    if ! nvidia-ctk cdi generate --device-name-strategy=type-index --format=json >/etc/cdi/nvidia.json; then
        rm -f /etc/cdi/nvidia.json
    fi

    # Ignore welcome warnings
    ${PREFIX} podman version
}

# Copy podman containers configuration file
if which podman; then
    mkdir -p "${HOME}/.config/containers"
    rm -rf \
        "${HOME}/.config/containers/containers.conf" \
        "${HOME}/.config/containers/storage.conf" \
        ${HOME}/.local/share/containers

    # Patch rootless containers
    cp /etc/containers/podman-containers.conf "${HOME}/.config/containers/containers.conf"
    cp /etc/containers/storage.conf "${HOME}/.config/containers/storage.conf"

    # Initialize rootless podman, without modifying at all
    if _reset_podman; then
        exec true
    fi

    # Cleanup old DB, resetting database static dir
    ${PREFIX} rm -f "/var/lib/containers/db.sql"

    # Initialize rootless podman, with cleaning DB
    if _reset_podman; then
        exec true
    fi

    # Cleanup containers
    ${PREFIX} rm -rf /var/lib/containers/*

    # Initialize rootless podman, with cleaning ALL
    _reset_podman || true
fi
