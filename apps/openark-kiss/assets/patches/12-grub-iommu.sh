#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Kernel Configuration
# Enable IOMMU

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

{{- if has "org.ulagbulag.io/vm/kubevirt" .Values.features }}

mkdir -p /etc/default/grub.d/
cat <<EOF >/etc/default/grub.d/20-iommu.cfg
GRUB_CMDLINE_LINUX="\${GRUB_CMDLINE_LINUX} amd_iommu=on intel_iommu=on iommu=pt"
EOF

{{- end }}
