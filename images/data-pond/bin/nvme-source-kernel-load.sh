#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Load NVMe Kernel Modules

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# nvme
modprobe nvme num_p2p_queues=1
modprobe nvme-tcp

# nvmet
modprobe nvmet
modprobe nvmet-tcp

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
    )
    index=$((index + 1))
}

# TCP
NVME_FABRIC_TCP_FAMILY='ipv4'
NVME_FABRIC_TCP_ADDRESS="${POD_IP}"
NVME_FABRIC_TCP_PORT="${NVME_FABRIC_TCP_PORT:-4420}"
add_port 'tcp'
