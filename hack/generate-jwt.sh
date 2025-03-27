#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Configuration                                         #
###########################################################

# Client ID as first argument
if [ -z "${CLIENT_ID}" ]; then
    echo 'No such environment variable: CLIENT_ID' >&2
    exit 1
fi

# File path of the private key as second argument
if [ -z "${PRIVATE_KEY_PATH}" ]; then
    echo 'No such environment variable: PRIVATE_KEY_PATH' >&2
    exit 1
fi
PRIVATE_KEY=$(cat "${PRIVATE_KEY_PATH}")

###########################################################
#   Utilities                                             #
###########################################################

function b64enc() {
    openssl base64 |
        tr -d '=' |
        tr '/+' '_-' |
        tr -d '\n'
}

###########################################################
#   Generate JWT                                          #
###########################################################

now=$(date +%s)
iat=$((${now} - 60))  # Issues 60 seconds in the past
exp=$((${now} + 600)) # Expires 10 minutes in the future

header_json='{
    "typ": "JWT",
    "alg": "RS256"
}'
# Header encode
header="$(echo -n "${header_json}" | b64enc)"

payload_json='
    "iat": '"${iat}"',
    "exp": '"${exp}"',
    "iss": "'"${CLIENT_ID}"'"
'
if [ ! -z "${EXTRA_PAYLOAD_JSON}" ]; then
    payload_json="${payload_json},${EXTRA_PAYLOAD_JSON}"
fi
# Payload encode
payload="$(echo -n "{${payload_json}}" | b64enc)"

# Signature
header_payload="${header}.${payload}"
signature="$(
    openssl dgst -sha256 -sign <(echo -n "${PRIVATE_KEY}") \
        <(echo -n "${header_payload}") | b64enc
)"

# Create JWT
JWT="${header_payload}.${signature}"
exec echo "${JWT}"
