#!/bin/bash
# Copyright (c) 2022-2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Execute a specific kubespray playbook in the cluster.

# Prehibit errors
set -e -o pipefail

function cleanup() {
    if [ -d "${WORKDIR}" ]; then
        cd - >/dev/null
        rm -rf "${WORKDIR}"
    fi
}

function terminate() {
    cleanup
    exec true
}

trap -- 'terminate' SIGINT
trap -- 'terminate' SIGTERM

###########################################################
#   Define Console Logger                                 #
###########################################################

function __log() {
    local level=$1
    local content=$2

    local reset='\033[0m'
    case "x${level}" in
    'xPATCH')
        local color='\033[35m'
        local important=0
        ;;
    'xSKIP')
        local color='\033[33m'
        local important=0
        ;;
    'xINFO')
        local color='\033[1;92m'
        local important=1
        ;;
    'xDONE')
        local color='\033[1;94m'
        local important=0
        ;;
    'xWARN')
        local color='\033[1;93m'
        local important=1
        ;;
    'xERROR')
        local color='\033[1;91m'
        local important=1
        ;;
    *)
        local color="${reset}"
        local important=0
        ;;
    esac

    local msg="${color} - ${content}${reset}\n"
    if [ "x${important}" == 'x1' ]; then
        local divider='================================================================================'
        local msg="${divider}\n${msg}"
    fi

    if [ "x${level}" == 'xERROR' ]; then
        printf "${msg}" >&2
        cleanup
        exit 1
    else
        printf "${msg}"
    fi
}

###########################################################
#   Main Function                                         #
###########################################################

# Define a main function
function main() {
    # Parse arguments
    local base_repo="$1"

    # Check base repository
    if [ ! -f "${base_repo}/values.yaml" ]; then
        __log 'ERROR' "No such repository: ${base_repo}"
    fi

    # Create a temporary directory
    local BASEDIR="$(realpath "$(dirname "$(dirname "$0")")")"
    export WORKDIR="$(mktemp -d)"
    chmod 700 "${WORKDIR}"
    cd "${WORKDIR}"

    # Merge cluster values
    yq eval-all '. as $item ireduce ({}; . * $item )' \
        "${BASEDIR}/values.yaml" \
        "${BASEDIR}/${base_repo}/values.yaml" \
        >./values.yaml

    # Begin building kubespray inventory
    mkdir inventory
    cd inventory

    # Get KISS Kubespray configurations
    cat "${BASEDIR}/apps/openark-kiss/values.yaml" | yq '.kubespray' >./all.yaml

    # Register bootstrapper node(s)
    local node_name="$(cat ../values.yaml | yq '.bootstrapper.node.name')"
    echo '{}' |
        yq ".all.hosts.${node_name}.ansible_host = \"$(cat ../values.yaml | yq '.bootstrapper.network.address.ipv4')\"" |
        yq ".all.hosts.${node_name}.ansible_host_key_checking = false" |
        yq ".all.hosts.${node_name}.ansible_ssh_host = \"$(cat ../values.yaml | yq '.bootstrapper.network.address.ipv4')\"" |
        yq ".all.hosts.${node_name}.ansible_ssh_port = 22" |
        yq ".all.hosts.${node_name}.ansible_ssh_user = \"$(cat ../values.yaml | yq '.kiss.auth.ssh.username')\"" |
        yq ".all.hosts.${node_name}.ip = \"$(cat ../values.yaml | yq '.bootstrapper.network.address.ipv4')\"" |
        yq ".all.hosts.${node_name}.name = \"${node_name}\"" |
        yq ".etcd.hosts.${node_name} = {}" |
        yq ".k8s_cluster.children.kube_control_plane = {}" |
        yq ".k8s_cluster.children.kube_node = {}" |
        yq ".kube_control_plane.hosts.${node_name} = {}" |
        yq ".kube_node.hosts.${node_name} = {}" |
        cat >./hosts.yaml
    unset node_name

    # Complete building kubespray inventory
    cd - >/dev/null

    # Begin building SSH inventory
    mkdir ssh
    chmod 700 ssh
    cd ssh

    # Get SSH private key
    cat ../values.yaml | yq -r '.kiss.auth.ssh.key.private' >./key
    chmod 400 ./key

    # Get SSH public key
    cat ../values.yaml | yq -r '.kiss.auth.ssh.key.public' >./key.pub

    # Complete building SSH inventory
    cd - >/dev/null

    # Deploy a k8s cluster
    docker run --rm \
        --mount type=bind,source="${WORKDIR}/inventory/",dst=/inventory \
        --mount type=bind,source="${WORKDIR}/ssh/",dst=/root/.ssh,readonly \
        --net host \
        "$(cat ./values.yaml | yq '.kiss.image.repo'):$(cat ./values.yaml | yq '.kiss.image.tag')" \
        ansible-playbook \
        --become \
        --become-user 'root' \
        --extra-vars '@/inventory/all.yaml' \
        --inventory '/inventory/hosts.yaml' \
        --private-key '/root/.ssh/key' \
        ${@:2}

    # Cleanup
    cleanup
}

# Execute main function
main $@
