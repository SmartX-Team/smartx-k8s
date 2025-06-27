#!/usr/bin/env bash
# Copyright (c) 2023-2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

function _reset_podman() {
    # Initialize rootless podman
    podman system migrate

    # Generate a CDI specification that refers to all NVIDIA devices
    if ! nvidia-ctk cdi generate --device-name-strategy=type-index --format=json >/etc/cdi/nvidia.json; then
        rm -f /etc/cdi/nvidia.json
    fi

    # Ignore welcome warnings
    podman version
}

# Copy podman containers configuration file
if which podman; then
    mkdir -p "${HOME}/.config/containers"
    rm -rf "${HOME}/.config/containers/containers.conf"

    # Patch rootless containers
    if [ "x$(id -u)" != 'x0' ]; then
        cp /etc/containers/podman-containers.conf "${HOME}/.config/containers/containers.conf"
    fi

    # Initialize rootless podman, without modifying at all
    if _reset_podman; then
        exec true
    fi

    # Cleanup old DB, resetting database static dir
    if [ "x$(id -u)" == 'x0' ]; then
        rm -f /var/lib/containers/db.sql
    else
        rm -f "${HOME}/.local/share/containers/db.sql"
    fi

    # Initialize rootless podman, with cleaning DB
    if _reset_podman; then
        exec true
    fi

    # Cleanup containers
    if [ "x$(id -u)" == 'x0' ]; then
        rm -rf /var/lib/containers/*
    else
        rm -rf ${HOME}/.local/share/containers/*
    fi

    # Initialize rootless podman, with cleaning ALL
    _reset_podman || true
fi
