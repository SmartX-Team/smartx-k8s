#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Utilities                                             #
###########################################################

function get_ip() {
    kubectl get box "$1" \
        --output jsonpath \
        --template '{.status.access.primary.address}'
}

function get_name() {
    alias="$(
        kubectl get box \
            --selector "dash.ulagbulag.io/alias=$1" \
            --output jsonpath \
            --template '{.items[].metadata.name}'
    )"
    if [ "x${alias}" == 'x' ]; then
        echo -n "$1"
    else
        echo -n "${alias}"
    fi
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

function usage() {
    echo "Usage: $0 [box] [command [argument ...]]" >&2
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

    alias="$1"
    box="$(get_name "${alias}")"
    ip="$(get_ip "${box}")"
    if [ "x${ip}" == 'x' ]; then
        echo "* Not Ready: ${box}" >&2
        exit 1
    fi

    ssh -i "${ssh_key}" \
        -o StrictHostKeyChecking=no \
        -o UserKnownHostsFile=/dev/null \
        "${ssh_user}@${ip}" ${@:2}

    rm -f "${ssh_key}"
}

main $@
