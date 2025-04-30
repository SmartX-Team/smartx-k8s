#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# APT Packages Configuration
# Install Dependencies

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

apt-get install --no-install-recommends --no-install-suggests -y \
{{- range $_ := .Values.packages.ubuntu.requires }}
    {{ . | quote }} \
{{- end }}
{{- if eq "ubuntu" .Values.kiss.os.dist }}
{{- if eq "edge" .Values.kiss.os.kernel }}
    {{ printf "linux-generic-hwe-%s" .Values.kiss.os.version | quote }} \
{{- else if eq "stable" .Values.kiss.os.kernel }}
    {{ printf "linux-generic" | quote }} \
{{- else }}
{{- fail ( printf "Unsupported kernel type: %s" .Values.kiss.os.kernel ) }}
{{- end }}
{{- else }}
{{- fail "Internal error: Ubuntu OS is not enabled" }}
{{- end }}
    && true
