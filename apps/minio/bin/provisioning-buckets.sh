#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# MinIO Provisioning Script Entrypoint
# Configure Buckets

# Prehibit errors
set -e -o pipefail

{{ range $_ := $.Values.buckets }}

{{- /*
Set default values
*/}}
{{- $name := print $.Values.bucketNamePrefix .name }}
{{- $_ := set . "anonymous" ( .anonymous | default dict ) }}
{{- $_ := set .anonymous "read" ( .anonymous.read | default false ) }}
{{- $_ := set .anonymous "write" ( .anonymous.write | default false ) }}
{{- $_ := set . "quota" ( .quota | default "" ) }}

# Create bucket: {{ $name }}
mc mb "provisioning/{{ $name }}" \
    --disable-pager \
    --ignore-existing \
    --no-color

# Set bucket quota: {{ $name }} -> {{ .quota | default "Not set" }}
{{- if empty .quota }}
mc quota clear "provisioning/{{ $name }}" \
    --disable-pager \
    --no-color
{{- else }}
mc quota set "provisioning/{{ $name }}" \
    --disable-pager \
    --no-color \
    --size {{ .quota | quote }}
{{- end }}

{{- if and .anonymous.ready .anonymous.write }}
{{- $_ := set . "permission" "public" }}
{{- else if and .anonymous.ready ( not .anonymous.write ) }}
{{- $_ := set . "permission" "download" }}
{{- else if and ( not .anonymous.ready ) .anonymous.write }}
{{- $_ := set . "permission" "upload" }}
{{- else }}
{{- $_ := set . "permission" "private" }}
{{- end }}
# Set bucket anonymous access: {{ $name }} -> {{ .permission }}
mc anonymous set {{ .permission }} "provisioning/{{ $name }}" \
    --disable-pager \
    --no-color

# mc replicate

{{- end }}

# Finished!
exec true
