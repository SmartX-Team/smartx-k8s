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
IO_SOURCES="${IO_SOURCES:-nvme}"
IO_TARGET="${IO_TARGET:-nvme}"
NVME_DRIVER="${NVME_DRIVER:-kernel}"

function terminate() {
    # Stop controller
    if [ "x${pid_controller}" != 'x' ]; then
        kill "${pid_controller}" || true
        wait "${pid_controller}" || true
    fi

    "$(dirname "$0")/${IO_TARGET}-target.sh" >&2
    for source in $(echo "${IO_SOURCES}" | tr ',' '\n'); do
        driver="$(cat "${source}" | tr a-z A-Z)_DRIVER"
        "$(dirname "$0")/${source}-target-cleanup.sh" >&2
        "$(dirname "$0")/${source}-source-${!driver}-cleanup.sh" >&2
        "$(dirname "$0")/${source}-source-${!driver}-unload.sh" >&2
    done
    exec true
}

trap -- 'terminate' SIGINT
trap -- 'terminate' SIGTERM

# Load driver
terminate
for source in $(echo "${IO_SOURCES}" | tr ',' '\n'); do
    driver="$(cat "${source}" | tr a-z A-Z)_DRIVER"
    "$(dirname "$0")/${source}-source-${!driver}-load.sh" >&2
done

# Discover sources
for source in $(echo "${IO_SOURCES}" | tr ',' '\n'); do
    driver="$(cat "${source}" | tr a-z A-Z)_DRIVER"
    "$(dirname "$0")/${source}-source-${!driver}-discover.sh" >&2
done

# Load controller
data_pond_controller &
declare -ig pid_controller="$!"

# Unload controller
wait "${pid_controller}" || true

# Unload driver
terminate
