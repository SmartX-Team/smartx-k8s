#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Configure environment variables
if [[ ! -v DATA_POND_IO_SOURCES ]]; then
    export DATA_POND_IO_SOURCES="nvme"
fi
export DATA_POND_IO_TARGETS="${DATA_POND_IO_TARGETS:-nvme}"
export NVME_DRIVER="${NVME_DRIVER:-kernel}"

# Mount ConfigFS
if ! cat /proc/mounts | grep -Posq '^configfs +/sys/kernel/config +configfs'; then
    mount configfs -t configfs /sys/kernel/config
fi

function terminate() {
    # Stop controller
    if [ "x${pid_controller}" != 'x' ]; then
        kill "${pid_controller}" || true
        wait "${pid_controller}" || true
    fi

    # Unload driver
    for target in $(echo "${DATA_POND_IO_TARGETS}" | tr ',' '\n'); do
        "$(dirname "$0")/${target}-target-unload.sh" >&2
    done
    for source in $(echo "${DATA_POND_IO_SOURCES}" | tr ',' '\n'); do
        driver="$(echo "${source}" | tr a-z A-Z)_DRIVER"
        "$(dirname "$0")/${source}-source-${!driver}-unload.sh" >&2
    done
}

trap -- 'terminate' SIGINT
trap -- 'terminate' SIGTERM

# Load driver
terminate
for source in $(echo "${DATA_POND_IO_SOURCES}" | tr ',' '\n'); do
    driver="$(echo "${source}" | tr a-z A-Z)_DRIVER"
    "$(dirname "$0")/${source}-source-${!driver}-load.sh" >&2
done
for target in $(echo "${DATA_POND_IO_TARGETS}" | tr ',' '\n'); do
    "$(dirname "$0")/${target}-target-load.sh" >&2
done

# Load node controller
data-pond &
declare -ig pid_controller="$!"

# Unload node controller
wait "${pid_controller}" || true

# Unload driver
terminate
