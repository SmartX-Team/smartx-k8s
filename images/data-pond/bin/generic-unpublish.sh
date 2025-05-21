#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Unpublish a volume

# Prehibit errors
set -e -o pipefail

# Parse inputs
inputs="$(cat | jq)"

# Unpublish a volume
exec umount "$(echo "${inputs}" | jq -r '.target_path')"
