#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Configuration                                         #
###########################################################

ACCESS_TOKEN="$("$(dirname "$0")/github-generate-access-token.sh")"

if [ -z "${REPO_OWNER}" ]; then
    echo 'No such environment variable: REPO_OWNER' >&2
    exit 1
fi
if [ -z "${REPO_NAME}" ]; then
    echo 'No such environment variable: REPO_NAME' >&2
    exit 1
fi

if [ -z "${COMMIT_SHA}" ]; then
    echo 'No such environment variable: COMMIT_SHA' >&2
    exit 1
fi

# Can be one of: queued, in_progress, completed, waiting, requested, pending
if [ -z "${CHECK_STATUS}" ]; then
    echo 'No such environment variable: CHECK_STATUS' >&2
    exit 1
fi
# Can be one of: action_required, cancelled, failure, neutral, success, skipped, stale, timed_out
if [ -z "${CHECK_CONCLUSION}" ]; then
    echo 'No such environment variable: CHECK_CONCLUSION' >&2
    exit 1
fi

###########################################################
#   Generate Access Token                                 #
###########################################################

output='{
    "title": "Mighty Readme report",
    "summary": "Hello Summary",
    "text": "# Hello World!"
}'
payload='{
    "name": "Example context",
    "head_sha": "'"${COMMIT_SHA}"'",
    "status": "'"${CHECK_STATUS}"'",
    "conclusion": "'"${CHECK_CONCLUSION}"'",
    "output": '"${output}"'
}'

exec curl -s -X POST \
    -d "${payload}" \
    -H "Accept: application/vnd.github+json" \
    -H "Authorization: Bearer ${ACCESS_TOKEN}" \
    "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/check-runs"
