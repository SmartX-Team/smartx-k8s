#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Advanced Network configuration
# Add OpenARK KISS Notifier Script

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

{{- if has "org.ulagbulag.io/bare-metal-provisioning/kiss" .Values.features }}

mkdir -p /etc/systemd/system/multi-user.target.wants/
cat <<EOF >/etc/systemd/system/notify-new-box.service
{{ tpl ( .Files.Get "systemd/notify-new-box.service" ) $ | replace "$" "\\$" }}
EOF
ln -sf /etc/systemd/system/notify-new-box.service /etc/systemd/system/multi-user.target.wants/notify-new-box.service

cat <<EOF >/usr/local/bin/notify-new-box.sh
{{ tpl ( .Files.Get "bin/notify-new-box.sh" ) $ | replace "$" "\\$" }}
EOF
chmod 550 /usr/local/bin/notify-new-box.sh

{{- end }}
