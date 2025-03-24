#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Kernel Configuration
# Disable ASPM (Active-State Power Management)

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

{{- if or
    ( has "org.ulagbulag.io/vm/kubevirt" .Values.features )
    ( eq "performance" .Values.optimization.profile )
}}
{{- $_ := set $.Values.optimization "aspm" "off" }}
{{- else }}
{{- $_ := set $.Values.optimization "aspm" "default" }}
{{- end }}

mkdir -p /etc/default/grub.d/
cat <<EOF >/etc/default/grub.d/20-aspm.cfg
GRUB_CMDLINE_LINUX="\${GRUB_CMDLINE_LINUX} pcie_aspm={{ $.Values.optimization.aspm }}"
EOF

