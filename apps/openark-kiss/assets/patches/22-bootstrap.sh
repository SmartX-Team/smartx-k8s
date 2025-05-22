#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Kubernetes Cluster Configuration
# Bootstrap a cluster in the first boot

# Prehibit errors
set -e -o pipefail

{{- if .Values.cluster.standalone }}

mkdir -p /etc/systemd/system/multi-user.target.wants/
cat <<EOF >/etc/systemd/system/smartx-k8s-bootstrap.service
{{ tpl ( .Files.Get "systemd/smartx-k8s-bootstrap.service" ) $ | replace "$" "\\$" }}
EOF
ln -sf /etc/systemd/system/smartx-k8s-bootstrap.service /etc/systemd/system/multi-user.target.wants/smartx-k8s-bootstrap.service

cat <<EOF >/usr/local/bin/smartx-k8s-bootstrap.sh
{{ tpl ( .Files.Get "bin/bootstrap.sh" ) $ | replace "$" "\\$" }}
EOF
chmod 550 /usr/local/bin/smartx-k8s-bootstrap.sh

{{- end }}
