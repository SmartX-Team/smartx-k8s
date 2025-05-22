#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Greeting Screen Configuration
# Add a daemon

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

{{- if has "org.ulagbulag.io/desktop-environment/vine" .Values.features }}

mkdir -p /etc/systemd/system/getty.target.wants/
cat <<EOF >/etc/systemd/system/openark-vine-greeter.service
{{ tpl ( .Files.Get "systemd/openark-vine-greeter.service" ) $ | replace "$" "\\$" }}
EOF

{{- if not .Values.cluster.standalone }}
ln -sf /etc/systemd/system/openark-vine-greeter.service /etc/systemd/system/getty.target.wants/openark-vine-greeter.service
{{- end }}

cat <<EOF >/usr/local/bin/openark-vine-greeter.sh
{{ .Files.Get "bin/openark-vine-greeter.sh" | replace "$" "\\$" }}
EOF
chmod 550 /usr/local/bin/openark-vine-greeter.sh

{{- end }}
