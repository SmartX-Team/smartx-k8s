#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

###########################################################
#   Validate Project                                      #
###########################################################

helm template smartx . --debug >/dev/null
./apps/rook-ceph-provisioning/tests/run.sh

[[ ${ISSUE_52_BUDGET_GATE:-} == 1 ]] || exit 0
approved='images/openark/bin/rook-ceph-provision.sh images/openark/src/rook-ceph-blkid-probe.c apps/rook-ceph-provisioning/tests/run.sh apps/rook-ceph-provisioning/Chart.yaml apps/rook-ceph-provisioning/manifest.yaml apps/rook-ceph-provisioning/patches.yaml apps/rook-ceph-provisioning/templates/resources.yaml apps/openark-kiss/tasks/join/provision-rook-ceph.yaml apps/openark-kiss/tasks/join/main-worker.yaml apps/rook-ceph-cluster/manifest.yaml apps/rook-ceph-cluster/patches.yaml apps/rook-ceph-operator/manifest.yaml values.yaml images/openark/Containerfile hack/git-ci-validate.sh'
{
    git diff --numstat 04c593d4c28c10cc245e94ecda3191f9b6ea79e1
    while IFS= read -r path; do
        [[ $path == .omo/* ]] || printf '%s\t0\t%s\n' "$(awk 'END { print NR }' "$path")" "$path"
    done < <(git ls-files --others --exclude-standard)
} | awk -v approved="$approved" '
    BEGIN { split(approved, paths); for (i in paths) allow[paths[i]] = 1 }
    NF != 3 || $1 !~ /^[0-9]+$/ || $2 !~ /^[0-9]+$/ || !($3 in allow) { bad = 1; next }
    { seen[$3] = 1; n = $1 + $2; total += n; if ($3 ~ /^images\/openark\/(bin|src)\//) runtime += n; else if ($3 ~ /tests\/run.sh$/) tests += n; else if ($3 ~ /^apps\/rook-ceph-provisioning\//) chart += n; else if ($3 ~ /^apps\/openark-kiss\//) kiss += n; else wiring += n }
    END { for (path in allow) if (!(path in seen)) bad = 1; exit bad || runtime > 332 || tests > 360 || chart > 150 || kiss > 88 || wiring > 75 || total > 1000 }'
