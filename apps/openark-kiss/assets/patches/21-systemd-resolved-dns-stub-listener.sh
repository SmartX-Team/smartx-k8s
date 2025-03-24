#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Advanced Network configuration
# Deactivate systemd-resolved DNSStubListener

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

mkdir -p /etc/systemd/resolved.conf.d/

cat <<EOF >/etc/systemd/resolved.conf.d/99-systemd.conf
[Resolve]
DNSStubListener=no
EOF
