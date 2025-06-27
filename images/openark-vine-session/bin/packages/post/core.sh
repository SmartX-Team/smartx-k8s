#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Common Configuration

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

{{- if .Values.user.sudo }}
# Allow passwordless sudo command
echo {{ printf "%s ALL=(ALL) NOPASSWD: ALL" .Values.user.name | quote }} >/etc/sudoers.d/10-wheel
chmod 440 /etc/sudoers.d/10-wheel
{{- end }}

# Environment Variables Configuration
ln -sf /usr/local/bin /opt/bin
cat <<EOF >/etc/profile.d/path-local-bin.sh
# local binary path registration
export PATH=${PATH}:/usr/games
export PATH=${PATH}:/usr/local/bin
export PATH=${PATH}:/opt/bin
EOF

cat <<EOF >/etc/ld.so.conf.d/100-path-local-lib.conf
# local library path registration
/usr/local/lib
/usr/local/lib64
EOF
