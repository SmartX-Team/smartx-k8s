#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Generate JWT                                          #
###########################################################

if [ -z "${GITHUB_INSTALLATION_ID}" ]; then
    echo 'No such environment variable: GITHUB_INSTALLATION_ID' >&2
    exit 1
fi
if [ -z "${GITHUB_CLIENT_ID}" ]; then
    echo 'No such environment variable: GITHUB_CLIENT_ID' >&2
    exit 1
fi
if [ -z "${GITHUB_PRIVATE_KEY_PATH}" ]; then
    echo 'No such environment variable: GITHUB_PRIVATE_KEY_PATH' >&2
    exit 1
fi

GITHUB_JWT="$(
    CLIENT_ID="${GITHUB_CLIENT_ID}"
    PRIVATE_KEY_PATH="${GITHUB_PRIVATE_KEY_PATH}"
    "$(dirname "$0")/generate-jwt.sh"
)"

###########################################################
#   Login to GitHub                                       #
###########################################################

exec curl -s -X POST \
    --header "Accept: application/vnd.github+json" \
    --header "Authorization: Bearer ${GITHUB_JWT}" \
    --header "X-GitHub-Api-Version: 2022-11-28" \
    --url "https://api.github.com/app/installations/${GITHUB_INSTALLATION_ID}/access_tokens"
