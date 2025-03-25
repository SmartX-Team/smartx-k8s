#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# MinIO Provisioning Script Entrypoint

# Prehibit errors
set -e -o pipefail

# Register alias
mc alias set provisioning "${MINIO_SCHEME}://minio:9000" "${MINIO_ROOT_USER}" "${MINIO_ROOT_PASSWORD}"

function restart() {
    # Restart server
    mc admin service restart provisioning --wait --json

    # Avoid a race condition
    sleep 5
    until mc admin info provisioning >/dev/null; do
        sleep 1
    done
}

# Restart MinIO for clean configuration
restart

# Do Provisioning
"$(dirname "$0")/provisioning-buckets.sh"
"$(dirname "$0")/provisioning-openid.sh"

# Restart MinIO for applying
restart

# Finished!
exec true
