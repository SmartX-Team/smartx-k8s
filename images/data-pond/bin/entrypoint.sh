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
export DATA_POND_IO_SOURCES="${DATA_POND_IO_SOURCES:-nvme}"
export DATA_POND_IO_TARGET="${DATA_POND_IO_TARGET:-nvme}"
export NVME_DRIVER="${NVME_DRIVER:-kernel}"

# Execute global controller
if ! echo -n "${DATA_POND_SERVICES}" | grep -Posq '(^|,)node(,|$)'; then
    exec data-pond
fi

function terminate() {
    # Stop controller
    if [ "x${pid_controller}" != 'x' ]; then
        kill "${pid_controller}" || true
        wait "${pid_controller}" || true
    fi

    # Unload driver
    "$(dirname "$0")/${DATA_POND_IO_TARGET}-target-unload.sh" >&2
    for source in $(echo "${DATA_POND_IO_SOURCES}" | tr ',' '\n'); do
        driver="$(echo "${source}" | tr a-z A-Z)_DRIVER"
        if [ "${source}" != "${DATA_POND_IO_TARGET}" ] && [ -f "$(dirname "$0")/${source}-target-unload.sh" ]; then
            "$(dirname "$0")/${source}-target-unload.sh" >&2
        fi
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

# Load node controller
data-pond &
declare -ig pid_controller="$!"

# Unload node controller
wait "${pid_controller}" || true

# Unload driver
terminate
