#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# MinIO Provisioning Script Entrypoint
# Configure OpenID

# Prehibit errors
set -e -o pipefail

{{ range $_ := $.Values.idp.openid }}

{{- /*
Check whether the config already exists
*/}}
if mc idp openid info provisioning {{ .name | quote }} >/dev/null 2>/dev/null; then
    cmd='update'
else
    cmd='add'
fi

# Set OpenID: {{ .name }}
mc idp openid "${cmd}" provisioning {{ .name | quote }} \
    --no-color \
    display_name={{ .title | default .name | quote }} \
    config_url={{ .configUrl | quote }} \
    client_id={{ .clientId | quote }} \
    client_secret={{ .clientSecret | quote }} \
    claim_name={{ .claimName | quote }} \
    scopes={{ .scopes | join "," | quote }}

{{- end }}

# Finished!
exec true
