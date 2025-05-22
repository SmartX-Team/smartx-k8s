#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Packages Configuration
# Install yq

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

{{- if .Values.cluster.standalone }}

curl -Lo '/usr/local/bin/yq' 'https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64'
chmod a+x '/usr/local/bin/yq'

{{- end }}
