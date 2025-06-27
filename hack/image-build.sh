#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Configuration                                         #
###########################################################

export ROOT="${ROOT:-$(pwd)}"

IMAGE_HOME="$($(dirname "$0")/image-prepare.sh ${@:1})"

###########################################################
#   Auto-detect Container Runtime                         #
###########################################################

if podman ps >/dev/null 2>/dev/null; then
    CONTAINER_RUNTIME="podman"
    CONTAINER_RUNTIME_EXTRA_ARGS="--format docker"
elif docker ps >/dev/null 2>/dev/null; then
    CONTAINER_RUNTIME="docker"
    CONTAINER_RUNTIME_EXTRA_ARGS=""
else
    echo "Container runtime not found: docker, podman" >&2
    exit 1
fi

###########################################################
#   Auto-detect metadata                                  #
###########################################################

CONFIGMAP_PATH="${IMAGE_HOME}/container-image/templates/configmap-containerfile.yaml"

BASE_IMAGE_REPO="$(
    cat ${CONFIGMAP_PATH} |
        yq '.metadata.annotations."images.ulagbulag.io/base.repo"'
)"
IMAGE_NAME="$(
    cat ${CONFIGMAP_PATH} |
        yq '.metadata.name'
)"
IMAGE_VERSION="$(
    cat ${CONFIGMAP_PATH} |
        yq '.metadata.labels."app.kubernetes.io/version"'
)"
eval EXTRA_ARGS=\""$(
    cat ${CONFIGMAP_PATH} |
        yq '.data.args'
)"\"

IMAGE_TAG="${BASE_IMAGE_REPO}/${IMAGE_NAME}:${IMAGE_VERSION}"

unset CONFIGMAP_PATH

###########################################################
#   Build a Containerfile                                 #
###########################################################

set +e +o pipefail
"${CONTAINER_RUNTIME}" build \
    --tag "${IMAGE_TAG}" \
    ${CONTAINER_RUNTIME_EXTRA_ARGS} \
    ${EXTRA_ARGS} \
    "${IMAGE_HOME}"
exit_code="$?"

###########################################################
#   Push the built image                                  #
###########################################################

if [ "x${exit_code}" == 'x0' ]; then
    "${CONTAINER_RUNTIME}" push "${IMAGE_TAG}"
    exit_code="$?"
fi

###########################################################
#   Cleanup                                               #
###########################################################

rm -rf "${IMAGE_HOME}"
exit "${exit_code}"
