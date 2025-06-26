#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Docker (Podman) Configuration

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

{{- if .Values.user.sudo }}
# Allow passwordless sudo command
echo {{ printf "%s ALL=(ALL) NOPASSWD: ALL" .Values.user.name | quote }} >/etc/sudoers.d/10-wheel
chmod 440 /etc/sudoers.d/10-wheel
{{- end }}
