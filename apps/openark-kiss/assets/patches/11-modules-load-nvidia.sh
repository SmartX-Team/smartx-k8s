#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Kernel Modules configuration
# Enable NVIDIA associated drivers

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

mkdir -p /etc/modules-load.d/

cat <<EOF >/etc/modules-load.d/10-gpu-nvidia-driver.conf
loop
i2c_core
ipmi_msghandler
EOF
