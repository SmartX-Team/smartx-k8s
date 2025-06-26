#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Configure blueman-applet

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

patch_src='/usr/lib/python3/dist-packages/blueman/plugins/applet/Networking.py'
if [ -f "${patch_src}" ]; then
    sed -i 's/^\( *\)d \= ErrorDialog(/\1raise result\n\0/g' "${patch_src}"
fi
