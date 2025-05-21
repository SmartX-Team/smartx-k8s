#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Unstage volumes

# Prehibit errors
set -e -o pipefail

# Parse inputs
inputs="$(cat | jq)"
echo "$inputs" >&2

# Unstage volumes
# FIXME: To be implemented!
exec false
