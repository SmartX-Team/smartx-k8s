#!/usr/bin/env bash
# Copyright (c) 2025-2026 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
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

if [ "x${CONTAINER_RUNTIME}" == 'x' ]; then
    if podman ps >/dev/null 2>/dev/null; then
        CONTAINER_RUNTIME="podman buildx"
        CONTAINER_RUNTIME_EXTRA_ARGS="--format docker"
    elif buildah ps >/dev/null 2>/dev/null; then
        CONTAINER_RUNTIME="buildah"
        CONTAINER_RUNTIME_EXTRA_ARGS="--format docker"
    elif docker ps >/dev/null 2>/dev/null; then
        CONTAINER_RUNTIME="docker buildx"
        CONTAINER_RUNTIME_EXTRA_ARGS=""
    else
        echo "Container runtime not found: docker, podman" >&2
        exit 1
    fi
fi
if [ "x${CONTAINER_RUNTIME_BUILD}" == 'x' ]; then
    CONTAINER_RUNTIME_BUILD="build"
fi
if [ "x${CONTAINER_RUNTIME_BUILD_PLATFORMS}" == 'x' ]; then
    CONTAINER_RUNTIME_BUILD_PLATFORMS="linux/amd64,linux/arm64"
fi

# Detect manifest mode
set +e +o pipefail
[ "x${CONTAINER_RUNTIME_BUILD}" == 'xbuild' ] && buildah manifest --help >/dev/null 2>/dev/null
USE_MANIFEST="$?"
set -e -o pipefail

###########################################################
#   Auto-detect metadata                                  #
###########################################################

CONFIGMAP_PATH="${IMAGE_HOME}/container-image/templates/configmap-containerfile.yaml"

BASE_IMAGE_REPO="$(
    cat ${CONFIGMAP_PATH} |
        yq -r '.metadata.annotations."images.ulagbulag.io/base.repo"'
)"
IMAGE_NAME="$(
    cat ${CONFIGMAP_PATH} |
        yq -r '.metadata.name'
)"
IMAGE_VERSION="$(
    cat ${CONFIGMAP_PATH} |
        yq -r '.metadata.labels."app.kubernetes.io/version"'
)"
eval EXTRA_ARGS=\""$(
    cat ${CONFIGMAP_PATH} |
        yq -r '.data.args'
)"\"

IMAGE_TAG="${BASE_IMAGE_REPO}/${IMAGE_NAME}:${IMAGE_VERSION}"
NUM_JOBS='4'

unset CONFIGMAP_PATH

###########################################################
#   Build a Containerfile                                 #
###########################################################

# Apply manifest mode for building
if [ "x${USE_MANIFEST}" = 'x0' ]; then
    CONTAINER_RUNTIME_EXTRA_ARGS="${CONTAINER_RUNTIME_EXTRA_ARGS} --jobs "$(( NUM_JOBS * "$(echo "${CONTAINER_RUNTIME_BUILD_PLATFORMS}" | sed 's/,/\n/g' | wc -l)" ))" --manifest ${IMAGE_TAG} --platform ${CONTAINER_RUNTIME_BUILD_PLATFORMS}"
else
    CONTAINER_RUNTIME_EXTRA_ARGS="${CONTAINER_RUNTIME_EXTRA_ARGS} --tag ${IMAGE_TAG}"
fi

# Create a manifest
if [ "x${USE_MANIFEST}" = 'x0' ]; then
    # Remove the old manifest
    if ${CONTAINER_RUNTIME} manifest exists "${IMAGE_TAG}" 2>/dev/null; then
        ${CONTAINER_RUNTIME} manifest rm "${IMAGE_TAG}" >/dev/null
    fi

    ${CONTAINER_RUNTIME} rmi "${IMAGE_TAG}" >/dev/null 2>/dev/null || true
    ${CONTAINER_RUNTIME} manifest create "${IMAGE_TAG}" --amend >/dev/null
fi

# Build it
set +e +o pipefail
${CONTAINER_RUNTIME} ${CONTAINER_RUNTIME_BUILD} \
    --security-opt 'seccomp=unconfined' \
    ${CONTAINER_RUNTIME_EXTRA_ARGS} \
    ${EXTRA_ARGS} \
    "${IMAGE_HOME}"
exit_code="$?"

###########################################################
#   Push the built image                                  #
###########################################################

if [ "x${exit_code}" == 'x0' ]; then
    if [ "x${USE_MANIFEST}" = 'x0' ]; then
        ${CONTAINER_RUNTIME} manifest push \
            --all \
            --format 'oci' \
            "${IMAGE_TAG}"
        exit_code="$?"
    else
        ${CONTAINER_RUNTIME} push "${IMAGE_TAG}"
        exit_code="$?"
    fi
fi

###########################################################
#   Cleanup                                               #
###########################################################

rm -rf "${IMAGE_HOME}"
exit "${exit_code}"
