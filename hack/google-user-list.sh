#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Configuration                                         #
###########################################################

if [ -z "${GOOGLE_DOMAIN}" ]; then
    echo 'No such environment variable: GOOGLE_DOMAIN' >&2
    exit 1
fi

outputs="$(mktemp -d)"
page_token=''

###########################################################
#   Browse Pages                                          #
###########################################################

while :; do
    # Crawl current page
    buf="$(mktemp -p "${outputs}")"
    curl -s -X GET \
        --header "Authorization: Bearer $("$(dirname "$0")/google-generate-access-token.sh")" \
        "https://admin.googleapis.com/admin/directory/v1/users?domain=${GOOGLE_DOMAIN}&pageToken=${page_token}" >"${buf}"

    # Point to the next page
    page_token="$(jq -r '.nextPageToken' "${buf}")"
    if [ "${page_token}" == 'null' ]; then
        break
    fi
done

###########################################################
#   Collect Data                                          #
###########################################################

echo '---'
jq -s '[ .[].users ] | add' $(find "${outputs}" -maxdepth 1 -type f) |
    jq 'del (.[].changePasswordAtNextLogin)' |
    jq 'del (.[].creationTime)' |
    jq 'del (.[].customerId)' |
    jq 'del (.[].etag)' |
    jq 'del (.[].id)' |
    jq 'del (.[].isDelegatedAdmin)' |
    jq 'del (.[].kind)' |
    jq 'del (.[].lastLoginTime)' |
    jq 'del (.[].nonEditableAliases)' |
    jq 'del (.[].suspensionReason)' |
    jq '[ to_entries[] | {
        apiVersion: "org.ulagbulag.io/v1alpha1",
        kind: "User",
            metadata: {
                name: ( .value.primaryEmail | split("@") | first ),
                annotations: {
                    "org.ulagbulag.io/email": ( .value.primaryEmail ),
                },
                labels: {
                    "org.ulagbulag.io/enabled": ( .value.suspended | not | tostring ),
                    "org.ulagbulag.io/name": "google",
                    "org.ulagbulag.io/kind": "google",
                    "org.ulagbulag.io/privileged": ( .value.isAdmin ),
                },
            },
        spec: ( .value | to_entries | sort_by(.key) | from_entries ),
    } ] | sort_by(.metadata.name)' |
    yq --prettyPrint '.[] | split_doc'

###########################################################
#   Cleanup                                               #
###########################################################

exec rm -rf "${outputs}"
