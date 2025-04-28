#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Greeting Screen
# Unload all graphics modules

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Check user
if [ "x$(id -u)" != 'x0' ]; then
    echo 'ERROR: Permission denied (root-only)'
    exec false
fi

modules=(
    'i915'
    'nouveau'
    'nvidia_drm'
    'nvidia_modeset'
    'nvidia_uvm'
    'nvidia'
)
for module in ${modules[@]}; do
    while [ -d "/sys/module/${module}" ]; do
        rmmod "${module}" || sleep 0.1
    done
done
