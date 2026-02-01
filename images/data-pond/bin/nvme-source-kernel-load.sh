#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Load NVMe transport Kernel Modules

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# nvmet
modprobe nvme num_p2p_queues=1
modprobe nvmet
modprobe nvmet-tcp

#######################################
# Add ports
#######################################

declare -ig index=$((1))

function toggle_subsystems() {
    for subsystem in $(
        find '/sys/kernel/config/nvmet/subsystems/' -mindepth 1 -maxdepth 1 -type d |
            sort
    ); do
        for namespace in $(
            find "${subsystem}/namespaces/" -mindepth 1 -maxdepth 1 -type d |
                sort
        ); do
            echo "$1" >"${namespace}/enable"
        done
    done
}

function add_port {
    port="/sys/kernel/config/nvmet/ports/${index}"
    mkdir -p "${port}"
    cd "${port}"

    # Disable all subsystems
    subsystems="$(ls ./subsystems/)"
    for subsystem in ${subsystems}; do
        rm "./subsystems/${subsystem}"
    done

    (
        type="$1"
        case "${type}" in
        'tcp')
            echo "${NVME_FABRIC_TCP_FAMILY}" >./addr_adrfam
            echo "${NVME_FABRIC_TCP_ADDRESS}" >./addr_traddr
            echo "${NVME_FABRIC_TCP_PORT}" >./addr_trsvcid
            echo 'tcp' >./addr_trtype
            ;;
        esac
    )
    index=$((index + 1))

    # Enable all subsystems
    for subsystem in ${subsystems}; do
        ln -s "/sys/kernel/config/nvmet/subsystems/${subsystem}" "./subsystems/${subsystem}"
    done
}

# TCP
NVME_FABRIC_TCP_FAMILY='ipv4'
NVME_FABRIC_TCP_ADDRESS="${POD_IP}"
add_port 'tcp'
