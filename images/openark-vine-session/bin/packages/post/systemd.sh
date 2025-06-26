#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# SystemD Configuration

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

INIT_JOURNALCTL_URL="https://raw.githubusercontent.com/gdraheim/docker-systemctl-replacement/master/files/docker/journalctl3.py"
INIT_SYSTEMCTL_URL="https://raw.githubusercontent.com/gdraheim/docker-systemctl-replacement/master/files/docker/systemctl3.py"

# Download user-level SystemD
curl -s "${INIT_JOURNALCTL_URL}" -o '/usr/bin/journalctl'
curl -s "${INIT_SYSTEMCTL_URL}" -o '/usr/bin/systemctl'

# Prepare SystemD services
rm -rf '/etc/systemd/system/multi-user.target.wants'
mkdir -p \
    '/etc/systemd/system/multi-user.target.wants' \
    '/etc/systemd/user/default.target.wants'
ln -sf \
    {{ printf "/usr/lib/systemd/user/%s.service" .Release.Name | quote }} \
    {{ printf "/etc/systemd/user/default.target.wants/%s.service" .Release.Name | quote }}

# Prepare dummy scripts
mkdir -p '/opt/scripts'
echo 'sleep infinity' >'/opt/scripts/entrypoint-desktop.sh'
chmod a+x /opt/scripts/*

# SystemD Configuration
chmod u+x /usr/bin/journalctl /usr/bin/systemctl
