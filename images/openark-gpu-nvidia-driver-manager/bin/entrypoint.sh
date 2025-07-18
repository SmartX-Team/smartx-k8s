#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Drain all pods using NVIDIA GPUs
kubectl get pods \
    --all-namespaces \
    --field-selector spec.nodeName=e4020aba-4b62-55fa-1362-1c697ad99df2 \
    --output json |
    jq -r '.items[]
        | select(any(.spec.containers[].resources.requests; .["nvidia.com/gpu"]))
        | "\(.metadata.namespace) \(.metadata.name)"' |
    while read namespace pod; do
        cmd="kubectl delete pod --namespace ${namespace} ${pod}"
        ${cmd} --grace-period 30 && continue ||
            ${cmd} --now && continue ||
            ${cmd} --force ||
            true # ignore errors
    done

exec echo 'Completed.'
