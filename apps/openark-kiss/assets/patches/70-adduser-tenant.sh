#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Desktop Environment Configuration
# Add user: tenant

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

TENANT_HOME="/opt/vdi/tenants/host"

mkdir -p "${TENANT_HOME}"
chmod 700 "${TENANT_HOME}"
if ! grep -Pq '^tenant:' /etc/passwd; then
    groupadd --gid "2000" "tenant"
    useradd --uid "2000" --gid "2000" \
        --groups "audio,cdrom,input,render,tty,video" \
        --home "${TENANT_HOME}" --shell "/bin/bash" \
        --non-unique "tenant"
fi
chown -R tenant:tenant "${TENANT_HOME}"
