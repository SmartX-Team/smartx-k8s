#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Install AI dev dependencies - NVIDIA DeepStream

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

DEEPSTREAM_REFERENCES_REPO_TAG="DS-6.4"
DEEPSTREAM_REFERENCES_REPO_URL="https://github.com/NVIDIA-AI-IOT/deepstream_reference_apps.git"
DEEPSTREAM_URL_DOWNLOAD="https://api.ngc.nvidia.com/v2/org/nvidia/resources/deepstream/versions"
DEEPSTREAM_VERSION_MAJOR="6"
DEEPSTREAM_VERSION_MINOR="4"
DEEPSTREAM_VERSION_PATCH="0"
DEEPSTREAM_VERSION_URL="https://raw.githubusercontent.com/NVIDIA-AI-IOT/deepstream_dockers/main/common/version"

# Generate a Bearer token
TOKEN=$(
    curl -s -u "\$oauthtoken:${NGC_CLI_API_KEY}" -H 'Accept:application/json' \
        'https://authn.nvidia.com/token?service=ngc&scope=group/ngc' |
            jq -r '.token'
)

# Get the latest version
DEEPSTREAM_VERSION="$(
    curl -s "${DEEPSTREAM_VERSION_URL}" |
        grep -Po '^version\=\K[0-9\.]+$'
)"

# Parse the version information
DEEPSTREAM_HOME="/opt/nvidia/deepstream/deepstream"
DEEPSTREAM_VERSION_MAJOR="${DEEPSTREAM_VERSION_MAJOR:-"$(echo "${DEEPSTREAM_VERSION}" | awk -F '.' '{print $1}')"}"
DEEPSTREAM_VERSION_MINOR="${DEEPSTREAM_VERSION_MINOR:-"$(echo "${DEEPSTREAM_VERSION}" | awk -F '.' '{print $2}')"}"
DEEPSTREAM_VERSION_PATCH="${DEEPSTREAM_VERSION_PATCH:-"$(echo "${DEEPSTREAM_VERSION}" | awk -F '.' '{print $3}')"}"
DEEPSTREAM_VERSION_RELEASE="${DEEPSTREAM_VERSION_MAJOR}.${DEEPSTREAM_VERSION_MINOR}"
DEEPSTREAM_VERSION_FULL="${DEEPSTREAM_VERSION_RELEASE}.${DEEPSTREAM_VERSION_PATCH}"
DEEPSTREAM_URL_DOWNLOAD="${DEEPSTREAM_URL_DOWNLOAD}/${DEEPSTREAM_VERSION_RELEASE}/files"
DEEPSTREAM_FILE_DOWNLOAD="$(
    curl -s "${DEEPSTREAM_URL_DOWNLOAD}" \
        --oauth2-bearer "${TOKEN}" |
            jq -r '.recipeFiles[].path' |
            grep -Po "deepstream-${DEEPSTREAM_VERSION_RELEASE}_${DEEPSTREAM_VERSION_FULL}-[0-9]*_$(dpkg --print-architecture).deb" |
            sort -rV |
            head -n1
)"

# Download
DEEPSTREAM_FILE="/opt/deepstream-sdk.deb"
curl -Lo "${DEEPSTREAM_FILE}" "${DEEPSTREAM_URL_DOWNLOAD}/${DEEPSTREAM_FILE_DOWNLOAD}" \
    --oauth2-bearer "${TOKEN}"

# Decompress the downloaded file
apt-get install -y "${DEEPSTREAM_FILE}"

# Install
cd "${DEEPSTREAM_HOME}"
sed -i 's/"rhel"/"rocky"/g' ./*.sh
./install.sh
rm -f *.sh
cd -

# Download the latest configuration files
DEEPSTREAM_MODELS_DIR="${DEEPSTREAM_HOME}/samples/configs/tao_pretrained_models"
DEEPSTREAM_SAMPLE_HOME="/opt/deepstream_reference_apps"
git clone "${DEEPSTREAM_REFERENCES_REPO_URL}" "${DEEPSTREAM_SAMPLE_HOME}" \
    --branch "${DEEPSTREAM_REFERENCES_REPO_TAG}" \
    --single-branch

cd "${DEEPSTREAM_SAMPLE_HOME}/deepstream_app_tao_configs/"
cp -a * "${DEEPSTREAM_MODELS_DIR}"
cd -

# Download the models
cd "${DEEPSTREAM_MODELS_DIR}"
./download_models.sh
cd -

# Change permissions for user-level modification
chown -R {{ printf "%d:%d" ( .Values.user.uid | int ) ( .Values.user.gid | int ) | quote }} "${DEEPSTREAM_HOME}/samples"

# Cleanup
rm -rf "${DEEPSTREAM_SAMPLE_HOME}"
rm -f "${DEEPSTREAM_FILE}"
