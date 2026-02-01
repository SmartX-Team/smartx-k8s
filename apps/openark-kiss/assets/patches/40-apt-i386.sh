#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Packages Configuration
# Enable i386 Architecture

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

if [ "x$(uname -m)" == 'xx86_64' ]; then
    dpkg --add-architecture i386
fi
