#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Utilities                                             #
###########################################################

function get_alias() {
    alias="$(
        kubectl get box "$1" \
            --output jsonpath \
            --template '{.metadata.labels.dash\.ulagbulag\.io/alias}'
    )"
    if [ "x${alias}" == 'x' ]; then
        echo -n "$1"
    else
        echo -n "${alias}"
    fi
}

function get_ip() {
    kubectl get box "$1" \
        --output jsonpath \
        --template '{.status.access.primary.address}'
}

function get_ssh_private_key() {
    file="$(mktemp)"
    chmod 1600 "${file}"
    kubectl --namespace kiss get secret kiss-config \
        --output jsonpath \
        --template '{.data.auth_ssh_key_id_ed25519}' |
        base64 --decode >"${file}"
    echo -n "${file}"
}

function get_ssh_username() {
    kubectl --namespace kiss get configmap kiss-config \
        --output jsonpath \
        --template '{.data.auth_ssh_username}'
}

function list_boxes() {
    kubectl get box --no-headers | awk '{print $1}'
}

function usage() {
    echo "Usage: $0 [command [argument ...]]" >&2
    exit 1
}

###########################################################
#   Main Function                                         #
###########################################################

function main() {
    if [ "$#" -lt 1 ]; then
        usage
    fi

    ssh_key="$(get_ssh_private_key)"
    ssh_user="$(get_ssh_username)"

    for box in $(list_boxes); do
        echo '---'
        echo 'name: '"$(get_alias "${box}")"
        echo -n 'status: '

        # Test status
        ip="$(get_ip "${box}")"
        if [ "x${ip}" == 'x' ]; then
            echo 'Not Ready'
            continue
        fi

        # Test connection
        if ! ping -c 1 -W 4 "${ip}" >/dev/null; then
            echo 'Unreachable'
            continue
        fi
        echo 'Ready'

        echo 'outputs: -'
        echo "${ssh_user}@${ip}" ${@:1}
        ssh -i "${ssh_key}" \
            -o StrictHostKeyChecking=no \
            -o UserKnownHostsFile=/dev/null \
            "${ssh_user}@${ip}" bash -c '"'"${@:1}"'"'
    done

    echo '****************************************'
    rm -f "${ssh_key}"
}

main $@
