#!/usr/bin/env bash
# Copyright (c) 2025-2026 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Configuration                                         #
###########################################################

ROOT="${ROOT:-$(pwd)}"

if [ "x${PRESET_URL}" == 'x' ]; then
    echo "No such environment variable: PRESET_URL" >&2
fi

###########################################################
#   Create a temporary workspace                          #
###########################################################

IMAGE_HOME="$(mktemp -d)"

# NOTE: Do NOT copy symbolic links as-is
cp -Lr "${ROOT}/apps/openark-kiss/." "${IMAGE_HOME}"

# Copy cluster-wide values
mkdir "${IMAGE_HOME}/clusters"
cp "${ROOT}/values.yaml" "${IMAGE_HOME}/clusters/00-default.yaml"
cp "${ROOT}/iso/values.yaml" "${IMAGE_HOME}/clusters/90-iso.yaml"

# Download preset
if echo "${PRESET_URL}" | grep -Posq '^(git@|https://)'; then
    git clone --depth=1 "${PRESET_URL}" "${IMAGE_HOME}/preset"
else
    cp -Lr "${PRESET_URL}" "${IMAGE_HOME}/preset"
fi
cp "${IMAGE_HOME}/preset/values.yaml" "${IMAGE_HOME}/clusters/50-preset.yaml"

# Build a template
IMAGE_NAME="$(cat 'Chart.yaml' | yq -r '.name')"
helm template "${IMAGE_NAME}" "${IMAGE_HOME}" \
    --output-dir "${IMAGE_HOME}" \
    $(find "${IMAGE_HOME}/clusters" -name '*.yaml' -exec echo "--values {}" \; | sort -V) >/dev/null

# Return the chart home
exec echo "${IMAGE_HOME}"
