#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# DBus Configuration

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

patch_src='/var/lib/dpkg/statoverride'
if [ -f "${patch_src}" ]; then
    sed -i '/^root \+messagebus .*$/ d' /var/lib/dpkg/statoverride
fi
