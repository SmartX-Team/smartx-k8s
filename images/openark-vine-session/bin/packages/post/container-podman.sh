#!/usr/bin/env bash
# Copyright (c) 2023-2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Docker (Podman) Configuration

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Rootless Docker (Podman) Configuration
chmod u+s /usr/bin/newuidmap /usr/bin/newgidmap
systemctl enable podman

# Enable Fuse storage
mkdir -p \
    /var/lib/shared/overlay-images \
    /var/lib/shared/overlay-layers \
    /var/lib/shared/vfs-images \
    /var/lib/shared/vfs-layers
touch \
    /var/lib/shared/overlay-images/images.lock \
    /var/lib/shared/overlay-layers/layers.lock \
    /var/lib/shared/vfs-images/images.lock \
    /var/lib/shared/vfs-layers/layers.lock

# Generate a CDI specification that refers to all NVIDIA devices
mkdir -p /etc/cdi/
chown -R {{ printf "%d:%d" ( .Values.user.uid | int ) ( .Values.user.gid | int ) | quote }} /etc/cdi/
