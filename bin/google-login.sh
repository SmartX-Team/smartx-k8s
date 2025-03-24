#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Generate JWT                                          #
###########################################################

if [ -z "${GOOGLE_CLIENT_ID}" ]; then
    echo 'No such environment variable: GOOGLE_CLIENT_ID' >&2
    exit 1
fi
if [ -z "${GOOGLE_PRIVATE_KEY_PATH}" ]; then
    echo 'No such environment variable: GOOGLE_PRIVATE_KEY_PATH' >&2
    exit 1
fi
if [ -z "${GOOGLE_SCOPE}" ]; then
    echo 'No such environment variable: GOOGLE_SCOPE' >&2
    exit 1
fi

EXTRA_PAYLOAD_JSON='
    "aud": "https://oauth2.googleapis.com/token",
    "scope": "'"${GOOGLE_SCOPE}"'"
'
if [ ! -z "${GOOGLE_SUB}" ]; then
    EXTRA_PAYLOAD_JSON="${EXTRA_PAYLOAD_JSON}",'
        "sub": "'"${GOOGLE_SUB}"'"
    '
fi

GOOGLE_JWT="$(
    export CLIENT_ID="${GOOGLE_CLIENT_ID}"
    export PRIVATE_KEY_PATH="${GOOGLE_PRIVATE_KEY_PATH}"
    export EXTRA_PAYLOAD_JSON="${EXTRA_PAYLOAD_JSON}"
    "$(dirname "$0")/generate-jwt.sh"
)"

###########################################################
#   Login to Google                                       #
###########################################################

exec curl -s -X POST \
    --header "Content-Type: application/x-www-form-urlencoded" \
    --data "grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer&assertion=${GOOGLE_JWT}" \
    --url "https://oauth2.googleapis.com/token"
