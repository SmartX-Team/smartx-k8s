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
{{- if .Values.cluster.standalone }}
    'ipcalc' \
    'tini' \
{{- end }}
{{- if or
    .Values.cluster.standalone
    ( has "org.ulagbulag.io/desktop-environment/vine" .Values.features )
}}
    'containerd' \
{{- end }}
    && true

{{- if or
    .Values.cluster.standalone
    ( has "org.ulagbulag.io/desktop-environment/vine" .Values.features )
}}
# Backup container runtime binary
for f_src in $(find /usr/bin -maxdepth 1 -name 'containerd*'); do
    f_dst="/usr/local/bin/$(basename "${f_src}")"
    if [ ! -f "${f_dst}" ]; then
        cp -a "${f_src}" "${f_dst}"
    fi
done
(
    f_src='/usr/bin/runc'
    f_dst="/usr/local/bin/$(basename "${f_src}")"
    if [ ! -f "${f_dst}" ]; then
        cp -a "${f_src}" "${f_dst}"
    fi
)

# Patch services
(
    f_src='/usr/lib/systemd/system/containerd.service'
    f_dst='/etc/systemd/system/containerd.service'
    if [ -f "${f_src}" ]; then
        sed 's/^\(ExecStart=\)\/usr\/bin\/containerd/\1\/usr\/local\/bin\/containerd/g' "${f_src}" >"${f_dst}"
    fi
    apt-get remove -y containerd runc
    apt-get autoremove -y
)
{{- end }}
