#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Configuration                                         #
###########################################################

ROOT="$(pwd)"
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
#   Create an temporary helm chart                        #
###########################################################

IMAGE_HOME=$(mktemp -d)

# NOTE: Do NOT copy symbolic links as-is
cp -Lr "${IMAGE_SRC}/." "${IMAGE_HOME}"

# Copy cluster-wide values
# TODO: add support for 3rd-party cluster values
mkdir "${IMAGE_HOME}/clusters"
cp -r "${ROOT}/values.yaml" "${IMAGE_HOME}/clusters/default.yaml"

# Copy batteries
cp -r "${IMAGES_HOME}/template/." "${IMAGE_HOME}"

# Rename Containerfile
mv "${IMAGE_HOME}/Containerfile" "${IMAGE_HOME}/template.containerfile"

# Build a template
helm template "${IMAGE_NAME}" "${IMAGE_HOME}" \
    --output-dir "${IMAGE_HOME}" >/dev/null

# Extract Containerfile
cat "${IMAGE_HOME}/container-image/templates/configmap-containerfile.yaml" |
    yq '.data.Containerfile' >"${IMAGE_HOME}/Containerfile"

# Return the chart home
exec echo "${IMAGE_HOME}"
