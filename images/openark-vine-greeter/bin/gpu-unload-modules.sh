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

# Unload external modules
modules=(
    'nvidia_drm'
    'nvidia_modeset'
    'nvidia_uvm'
    'nvidia'
)
completed='false'
while [ "${completed}" == 'false' ]; do
    completed='true'
    for module in ${modules[@]}; do
        if [ -d "/sys/module/${module}" ]; then
            rmmod "${module}" || (completed='false' && sleep 0.1)
        fi
    done
done
