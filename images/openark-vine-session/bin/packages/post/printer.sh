#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Printer Configuration

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

{{- if eq "archlinux" .Values.dist.kind }}
groupadd --gid "104" "lpadmin"
{{- end }}

echo 'a4'
mkdir -p \
    /etc/cups \
    /etc/cupshelpers \
    /var/log/cups \
    /usr/lib/cups \
    /var/cache/cups \
    /run/cups \
    /var/spool/cups
chown -R {{ printf "%d:lpadmin" ( .Values.user.uid | int ) | quote }} \
    /etc/cups \
    /etc/cupshelpers \
    /var/log/cups \
    /usr/lib/cups \
    /var/cache/cups \
    /run/cups \
    /var/spool/cups \
    >/etc/papersize
