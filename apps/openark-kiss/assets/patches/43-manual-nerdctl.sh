#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Packages Configuration
# Install Nerdctl

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Alias sudo
if [ "x$(id -u)" != 'x0' ] && which sudo >/dev/null 2>/dev/null; then
    SUDO="sudo"
else
    SUDO=""
fi

ARCH="$(dpkg --print-architecture)"
FILENAME=$(
    curl -sL "https://api.github.com/repos/containerd/nerdctl/releases/latest" |
        grep -Po "nerdctl-[0-9\.]+-linux-${ARCH}.tar.gz" |
        sort |
        uniq |
        tail -n1
)
VERSION="$(echo "${FILENAME}" | cut -d '-' -f 2)"

cd /tmp
curl -Lo "${FILENAME}" "https://github.com/containerd/nerdctl/releases/download/v${VERSION}/${FILENAME}"
tar -zxf "${FILENAME}" nerdctl
${SUDO} mv nerdctl /usr/local/bin/nerdctl
rm "${FILENAME}"

# Update nerdctl configuration
mkdir -p /etc/nerdctl
cat <<EOF >/etc/nerdctl/nerdctl.toml
debug             = false
debug_full        = false
address           = "unix:///var/run/containerd/containerd.sock"
namespace         = "k8s.io"
snapshotter       = "overlayfs"
cni_path          = "/opt/cni/bin"
cni_netconfpath   = "/etc/cni/net.d"
cgroup_manager    = "systemd"
data_root         = "/run/nerdctl"
hosts_dir         = ["/etc/containerd/certs.d"]
EOF
