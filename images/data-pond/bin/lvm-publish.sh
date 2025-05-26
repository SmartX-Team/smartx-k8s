#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Publish a LVM volume

# Prehibit errors
set -e -o pipefail

# Parse inputs
inputs="$(cat | jq)"

# Publish a volume
staging_target_path="$(echo "${inputs}" | jq -r '.staging_target_path')"
target_path="$(echo "${inputs}" | jq -r '.target_path')"
mkdir -p "${target_path}"
exec mount -o bind \
    "${staging_target_path}" \
    "${target_path}"
