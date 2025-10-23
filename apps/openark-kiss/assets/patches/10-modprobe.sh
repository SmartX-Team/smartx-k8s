#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Kernel Modules configuration
# Patch kernel module parameters

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

mkdir -p /etc/modprobe.d/

{{- range $path, $_ := $.Files.Glob "assets/modprobe.d/*.conf" }}
{{- $filename := base $path }}
cat <<EOF >{{ printf "/etc/modprobe.d/%s" $filename | quote }}
{{ tpl ( $.Files.Get $path ) $ | replace "\\" "\\\\" | replace "$" "\\$" | trim }}
EOF
{{- end }}
