#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Advanced Network configuration
# Disable Kernel Module: rndis_host

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

mkdir -p /etc/default/grub.d/
cat <<EOF >/etc/default/grub.d/10-blacklist-rndis_host.cfg
GRUB_CMDLINE_LINUX="\${GRUB_CMDLINE_LINUX} modprobe.blacklist=rndis_host"
EOF

cat <<EOF >/etc/modprobe.d/blacklist-rndis_host.conf
blacklist rndis_host
EOF
