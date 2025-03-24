#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Advanced Network configuration
# Deactivate systemd-resolved

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

mkdir -p /etc/NetworkManager/conf.d/

cat <<EOF >/etc/NetworkManager/conf.d/99-systemd.conf
[main]
dns=default
rc-manager=resolvconf
EOF
