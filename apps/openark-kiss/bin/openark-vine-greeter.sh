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
if nerdctl stop -t 5 "openark-vine-greeter" >/dev/null; then
    nerdctl rm -f "openark-vine-greeter" >/dev/null || true
fi

# Cleanup nerdctl data
rm -rf /var/lib/nerdctl

# Create a namespace
NAMESPACE='k8s.io'
if ! nerdctl namespace ls | grep -Posq "${NAMESPACE}"; then
    nerdctl namespace create "${NAMESPACE}" || true
fi

# TODO: 잘못된 종료가 아니라면 무한히 생성하기
# TODO: greeter.sh SIGTERM 받아들여서 안전하게 종료하기 (종료코드 0이 아닌 경우 getty@tty1.service 로 이동!)
exec nerdctl run \
    --cgroup-parent 'kubepods.slice' \
    --cgroupns host \
    --name "openark-vine-greeter" \
    --namespace "${NAMESPACE}" \
    --network host \
    --privileged \
    --restart no \
    --rm \
    --tmpfs /tmp \
    --user root \
    --volume /dev:/dev:ro \
    --volume /lib/modules:/lib/modules:ro \
    "${IMAGE}"
