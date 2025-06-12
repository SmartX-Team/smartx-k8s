#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Load request                                          #
###########################################################

SRC="$(mktemp)"
cat <&0 >"${SRC}"

###########################################################
#   Utilities                                             #
###########################################################

function ret_deny() {
    message="$1"

    exec echo '{
        "status": "Deny",
        "message": "'"${message}"'"
    }'
}

function ret_pass() {
    exec echo '{
        "status": "Pass"
    }'
}

function ret_patch() {
    message="$1"
    operations="$2"

    exec echo '{
        "status": "Patch",
        "operations": '"$(echo "${operations}" | jq)"'
    }'
}

###########################################################
#   Execute CI                                            #
###########################################################

ret_patch 'Mounted host filesystem' '[
    {
        "op": "add",
        "path": "/spec/containers/0/volumeMounts/-",
        "value": {
            "name": "host-root",
            "mountPath": "/host",
            "mountPropagation": "HostToContainer",
            "readOnly": true
        }
    },
    {
        "op": "add",
        "path": "/spec/hostNetwork",
        "value": true
    }
]'
