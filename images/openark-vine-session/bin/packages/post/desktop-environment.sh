#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Desktop Environment Configuration

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Add read-only shared directory
mkdir -p /opt/public/
chown -R {{ printf "%d:%d" ( .Values.user.uid | int ) ( .Values.user.gid | int ) | quote }} /opt/public/

# Install utilities
chmod -R a+x "${ADDONS_HOME}/bin"
for file in ${ADDONS_HOME}/share/applications/*.desktop; do
    ln -s "${file}" "/usr/share/applications/$(basename "${file}")"
done
for file in ${ADDONS_HOME}/share/autostart/*.desktop; do
    ln -s "${file}" "/etc/xdg/autostart/$(basename "${file}")"
done

# Add system groups
{{- if eq "archlinux" .Values.dist.kind }}
groupadd --gid "24" "cdrom"
{{- end }}

# Add a user
{{ .Values.dist.current.ldconfig.path | quote }}
groupadd \
    -g {{ printf "%d" ( .Values.user.gid | int ) | quote }} \
    -o {{ .Values.user.name | quote }}
useradd \
    -u {{ printf "%d" ( .Values.user.uid | int ) | quote }} \
    -g {{ printf "%d" ( .Values.user.gid | int ) | quote }} \
    -G {{ .Values.user.groups | join "," | quote }} \
    -s {{ printf "/bin/%s" .Values.user.shell }} \
    -m \
    -o {{ .Values.user.name | quote }}
printf {{ printf "%d:%d:65535" ( .Values.user.uid | int ) ( add ( .Values.user.uid | int ) 1 ) | quote }} >/etc/subuid
printf {{ printf "%d:%d:65535" ( .Values.user.gid | int ) ( add ( .Values.user.gid | int ) 1 ) | quote }} >/etc/subgid

# Configure XDG
mkdir -p "${XDG_RUNTIME_DIR}"
chmod 700 "${XDG_RUNTIME_DIR}"
chown {{ printf "%d:%d" ( .Values.user.uid | int ) ( .Values.user.gid | int ) | quote }} "${XDG_RUNTIME_DIR}"
