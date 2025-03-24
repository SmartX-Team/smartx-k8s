#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Advanced Network configuration
# Disable Power Saving Mode (iwlmvm)

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

cat <<EOF >/etc/modprobe.d/iwlmvm.conf
options iwlmvm power_scheme=1
EOF
