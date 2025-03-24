#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Graphics Driver Configuration
# Disable GSP Firmware
# NOTE: https://github.com/NVIDIA/dcgm-exporter/issues/84

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

cat <<EOF >/etc/modprobe.d/disable-nvidia-gsp-firmware.conf
options nvidia NVreg_EnableGpuFirmware=0
EOF
