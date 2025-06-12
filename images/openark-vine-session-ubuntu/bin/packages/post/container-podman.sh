#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Docker (Podman) Configuration

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

PODMAN_REPO="https://raw.githubusercontent.com/containers/image_build/main/podman"

# Docker (Podman) Configuration
curl -sL -o '/etc/containers/containers.conf' "${PODMAN_REPO}/containers.conf"
curl -sL -o '/etc/containers/podman-containers.conf' "${PODMAN_REPO}/podman-containers.conf"
chmod 644 '/etc/containers/containers.conf' '/etc/containers/podman-containers.conf'

## Rootless Docker (Podman) Configuration
sed -i '/^keyring/ d' /etc/containers/containers.conf
sed -i 's/^\[containers\]/\0\nkeyring=false/g' /etc/containers/containers.conf
sed -i '/^no_pivot_root/ d' /etc/containers/containers.conf
sed -i 's/^\[engine\]/\0\nno_pivot_root=true/g' /etc/containers/containers.conf
chmod u+s /usr/bin/newuidmap /usr/bin/newgidmap
systemctl enable podman
touch /etc/containers/nodocker

## chmod containers.conf and adjust storage.conf to enable Fuse storage.
mkdir -p /etc/containers/
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
mkdir -p \
    /var/lib/shared/overlay-images \
    /var/lib/shared/overlay-layers \
    /var/lib/shared/vfs-images \
    /var/lib/shared/vfs-layers \
    touch /var/lib/shared/overlay-images/images.lock \
    touch /var/lib/shared/overlay-layers/layers.lock \
    touch /var/lib/shared/vfs-images/images.lock \
    touch /var/lib/shared/vfs-layers/layers.lock

## generate a CDI specification that refers to all NVIDIA devices
mkdir -p /etc/cdi/
chown -R {{ printf "%d:%d" ( .Values.user.uid | int ) ( .Values.user.gid | int ) | quote }} /etc/cdi/

# Environment Variables Configuration
ln -sf /usr/local/bin /opt/bin
cat <<EOF >/etc/profile.d/path-local-bin.sh
# local binary path registration
export PATH=${PATH}:/usr/games
export PATH=${PATH}:/usr/local/bin
export PATH=${PATH}:/opt/bin
EOF
cat <<EOF >/etc/ld.so.conf.d/100-path-local-lib.conf
# local library path registration
/usr/local/lib
/usr/local/lib64
EOF
