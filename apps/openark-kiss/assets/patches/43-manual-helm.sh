#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Packages Configuration
# Install Helm

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

{{- if .Values.cluster.standalone }}

curl "https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3" | bash

{{- end }}
