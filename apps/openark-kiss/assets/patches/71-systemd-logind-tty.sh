#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Desktop Environment Configuration
# Limit the maximum number of TTYs to 1

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

_LOGIND="/etc/systemd/logind.conf"
sed -i 's/^\#\?\(NAutoVTs=\).*$/\11/g' "${_LOGIND}"
sed -i 's/^\#\?\(ReserveVT=\).*$/\11/g' "${_LOGIND}"

for i in {2..63}; do
    systemctl mask getty@tty${i}.service >/dev/null
done

# Autologin
{{- if .Values.cluster.standalone }}
mkdir -p /etc/systemd/system/getty@tty1.service.d/
cat <<EOF >/etc/systemd/system/getty@tty1.service.d/override.conf
[Service]
ExecStart=
ExecStart=-/sbin/agetty -a {{ .Values.kiss.auth.ssh.username | quote }} --noclear - \$TERM
EOF
{{- end }}
