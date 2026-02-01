#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Greeting Screen Configuration

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Cleanup old container
NAME='openark-vine-greeter'
if nerdctl stop -t 5 "${NAME}" >/dev/null; then
    nerdctl rm -f "${NAME}" >/dev/null || true
fi

# Cleanup nerdctl data
rm -rf /var/lib/nerdctl

# Create a namespace
NAMESPACE='k8s.io'
if ! nerdctl namespace ls | grep -Posq "${NAMESPACE}"; then
    nerdctl namespace create "${NAMESPACE}" || true
fi

# Register termination function
function terminate() {
    # Stop container
    if [ "x${pid_container}" != 'x' ]; then
        kill "${pid_container}" || true
        wait "${pid_container}" || true
    fi
    timeout 15 nerdctl stop "${NAME}" || true
    nerdctl kill "${NAME}" || true

    # Cleanup module
    if [ -d '/sys/bus/pci/drivers/nouveau' ]; then
        rmmod nouveau || true
    fi
    exec true
}

trap -- 'terminate' SIGINT
trap -- 'terminate' SIGTERM

# Load kernel headers
EXTRA_VOLUMES=()
for path in $(find /usr/src -maxdepth 1 -name 'linux-*'); do
    EXTRA_VOLUMES+="--volume ${path}:${path}:ro "
done

# Start a container
nerdctl run \
    --cgroup-parent 'kubepods.slice' \
    --cgroupns host \
    --name "${NAME}" \
    --namespace "${NAMESPACE}" \
    --network host \
    --privileged \
    --restart no \
    --rm \
    --tmpfs /tmp \
    --user root \
    --volume /dev:/dev:ro \
    --volume /lib/modules:/lib/modules:ro \
    ${EXTRA_VOLUMES[@]} \
    "${IMAGE}" &
declare -ig pid_container="$!"

wait "${pid_container}" || true
terminate
