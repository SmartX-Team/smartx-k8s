#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# APT Packages Configuration
# Upgrade System

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

exec apt-get upgrade -y
