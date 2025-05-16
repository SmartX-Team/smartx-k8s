#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Load NVMe tranport Kernel Modules

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# nvmet
modprobe nvme num_p2p_queues=1
modprobe nvmet
modprobe nvmet-tcp

#######################################
# Add subsystem
#######################################

if [ "x${NVME_SUBSYSTEM_NQN}" == 'x' ]; then
    echo 'No such environment variable: NVME_SUBSYSTEM_NQN'
    exec false
fi
SUBSYSTEM_HOME="/sys/kernel/config/nvmet/subsystems/${NVME_SUBSYSTEM_NQN}"
mkdir "${SUBSYSTEM_HOME}"
cd "${SUBSYSTEM_HOME}"
(
    echo 1 >./attr_allow_any_host
)

#######################################
# Add ports
#######################################

declare -ig index=$((1))

function add_port {
    port="/sys/kernel/config/nvmet/ports/${index}"
    mkdir "${port}"
    cd "${port}"
    (
        type="$1"
        case "${type}" in
        'tcp')
            echo "${NVME_FABRIC_TCP_FAMILY}" >./addr_adrfam
            echo "${NVME_FABRIC_TCP_ADDRESS}" >./addr_traddr
            echo "${NVME_FABRIC_TCP_PORT}" >./addr_trsvcid
            ;;
        esac
        echo 'tcp' >./addr_trtype

        # Add port to the subsystem
        ln -s "${SUBSYSTEM_HOME}" "./subsystems/${SUBSYSTEM}"
    )
    index=$((index + 1))
}

# TCP
NVME_FABRIC_TCP_FAMILY='ipv4'
NVME_FABRIC_TCP_ADDRESS="${POD_IP}"
NVME_FABRIC_TCP_PORT="${NVME_FABRIC_TCP_PORT:-4420}"
add_port 'tcp'
