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

mkdir -p /etc/containers/
touch /etc/containers/nodocker

# Configure containers.conf
cat <<EOF >/etc/containers/containers.conf
[containers]
keyring=false
netns="host"
userns="host"
ipcns="host"
utsns="host"
cgroupns="host"
cgroups="disabled"
log_driver = "k8s-file"

[engine]
no_pivot_root=true
cgroup_manager = "cgroupfs"
events_logger="file"
runtime="crun"
EOF
chmod 644 /etc/containers/containers.conf

# Configure podman-containers.conf
cat <<EOF >/etc/containers/podman-containers.conf
[containers]
volumes = [
    "/proc:/proc",
]
default_sysctls = []
EOF
chmod 644 /etc/containers/podman-containers.conf

# Configure storage.conf
cat <<EOF >/etc/containers/storage.conf
[storage]
driver = "overlay"
graphroot = "/var/lib/containers/storage"
runroot = "/run/containers/storage"

[storage.options]
additionalimagestores = [
    "/var/lib/shared",
]

[storage.options.pull_options]
enable_partial_images = "false"
ostree_repos = ""
use_hard_links = "false"

[storage.options.overlay]
mount_program = "/usr/bin/fuse-overlayfs"
mountopt = "nodev,fsync=0"

[storage.options.thinpool]
EOF

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
