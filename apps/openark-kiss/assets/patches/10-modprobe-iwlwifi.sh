#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Advanced Network configuration
# Disable Power Saving Mode (iwlwifi)

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

cat <<EOF >/etc/modprobe.d/iwlwifi.conf
options iwlwifi power_save=0
EOF
