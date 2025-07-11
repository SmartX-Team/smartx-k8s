#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Remove conflicted repositories

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

{{- if eq "debian" .Values.dist.current.kind }}
rm /etc/apt/sources.list.d/microsoft.list
{{- end }}
