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
    # Create a temporary directory
    local BASEDIR="$(realpath "$(dirname "$(dirname "$0")")")"
    export WORKDIR="$(mktemp -d)"
    chmod 700 "${WORKDIR}"

    # Download preset
    if [ "x${PRESET_URL}" != 'x' ]; then
        if echo "${PRESET_URL}" | grep -Posq '^(git@|https://)'; then
            git clone "${PRESET_URL}" "${WORKDIR}/preset"
        else
            cp -Lr "${PRESET_URL}" "${WORKDIR}/preset"
        fi
    fi

    # Check base repository
    if [ ! -f "${WORKDIR}/preset/values.yaml" ]; then
        __log 'ERROR' "No such repository: ${PRESET_URL}"
    fi

    # Merge cluster values
    local values_file="${WORKDIR}/values.yaml"
    yq eval-all '. as $item ireduce ({}; . * $item )' \
        "${BASEDIR}/values.yaml" \
        "${WORKDIR}/preset/values.yaml" \
        >"${values_file}"

    # Copy OpenARK KISS application
    mkdir "${WORKDIR}/apps"
    cp -r "${BASEDIR}/apps/openark-kiss" "${WORKDIR}/apps/openark-kiss"

    # Build a kubespray script
    local kubespray_bin="${WORKDIR}/kubespray.sh"
    helm template "${WORKDIR}/apps/openark-kiss" \
        --set 'cluster.standalone=true' \
        --values "${values_file}" |
        yq 'select(.metadata.name == "iso") | .data."kubespray.sh"' >"${kubespray_bin}"
    chmod 500 "${kubespray_bin}"

    # Execute kubespray
    "${kubespray_bin}" ${@:1}

    # Cleanup
    cleanup
}

# Execute main function
main $@
