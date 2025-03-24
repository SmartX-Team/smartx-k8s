#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Desktop Environment Configuration
# Udev Configuration
# Grant all TTY permissions

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

mkdir -p /etc/udev/rules.d/
cat <<EOF >/etc/udev/rules.d/50-tenant-tty.rules
SUBSYSTEM=="tty", OWNER:="tenant"
EOF
