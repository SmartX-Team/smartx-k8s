#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Configuration                                         #
###########################################################

ROOT="${ROOT:-$(pwd)}"
IMAGES_HOME="${ROOT}/images"
if [ ! -d "${IMAGES_HOME}" ]; then
    echo "Invalid images directory: ${IMAGES_HOME}" >&2
    exit 1
fi

if [ -z "${IMAGE_NAME}" ]; then
    echo "No such environment variable: IMAGE_NAME" >&2
    exit 1
fi

IMAGE_SRC="${IMAGES_HOME}/${IMAGE_NAME}"
if [ ! -f "${IMAGE_SRC}/Containerfile" ]; then
    echo "Invalid image directory: ${IMAGE_SRC}" >&2
    exit 1
fi

###########################################################
#   Create a temporary helm chart                         #
###########################################################

IMAGE_HOME="$(mktemp -d)"

# NOTE: Do NOT copy symbolic links as-is
cp -Lr "${IMAGE_SRC}/." "${IMAGE_HOME}"

# Copy cluster-wide values
mkdir "${IMAGE_HOME}/clusters"
cp "${ROOT}/values.yaml" "${IMAGE_HOME}/clusters/default.yaml"

# Download preset
if [ "x${PRESET_URL}" != 'x' ]; then
    if echo "${PRESET_URL}" | grep -Posq '^(git@|https://)'; then
        git clone "${PRESET_URL}" "${IMAGE_HOME}/preset"
    else
        cp -Lr "${PRESET_URL}" "${IMAGE_HOME}/preset"
    fi
fi

# Copy batteries
cp -r "${IMAGES_HOME}/template/." "${IMAGE_HOME}"

# Rename Containerfile
mv "${IMAGE_HOME}/Containerfile" "${IMAGE_HOME}/template.containerfile"

# Copy cluster-wide helm chart metadata
cat "${ROOT}/Chart.yaml" |
    yq '.name="container-image"' |
    yq '.description="Temporary helm chart for building Containerfile"' \
        >"${IMAGE_HOME}/Chart.yaml"

# Build a template
helm template "${IMAGE_NAME}" "${IMAGE_HOME}" \
    --output-dir "${IMAGE_HOME}" >/dev/null

# Extract Containerfile
cat "${IMAGE_HOME}/container-image/templates/configmap-containerfile.yaml" |
    yq '.data.Containerfile' >"${IMAGE_HOME}/Containerfile"

# Return the chart home
exec echo "${IMAGE_HOME}"
