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

ret_pass
# TODO: (1) nvidia.com/gpu 인식
# TODO: (2) 털어내고 동일수량만큼 GPU를 탈락시키는 initContainer daemon sidecar 생성 (Desktop 에선 어차피 1개만 인식)
# TODO: (3) 탈락된 GPU는 VM에서 인식할 수 있도록 패치
# TODO: (4) 위의 수정사항은 json_patch::Patch 구문에 맞게 반영
# ret_deny "Hello World!"

exec echo '{
    "uid": '"$(cat "${SRC}" | jq '.uid')"',
    "allowed": true,
    "status": {
        "status": "Success",
        "code": 200,
        "message": "Nothing to be changed"
    },
    "patch": null,
    "patchType": "JSONPatch",
    "warnings": []
}'
