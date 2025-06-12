#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# X11 Configuration

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

mkdir -p /tmp/.X11-unix
chmod 777 /tmp/.X11-unix
