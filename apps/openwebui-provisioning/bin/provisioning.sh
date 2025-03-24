#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Open WebUI Provisioning Script Entrypoint

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

function provisioning_model() {
    model_name="$1"
    access_control_profile="$2"

    # Convert access control profile
    case "${access_control_profile}" in
    'private')
        access_control='{
            "read": { "group_ids": [] },
            "write": { "group_ids": [] }
        }'
        ;;
    'public')
        access_control='null'
        ;;
    *)
        echo "Unknown access control profile: ${model_name} -> ${access_control_profile}" >&2
        exit 1
        ;;
    esac

    # Pull a model
    curl "http://open-webui-ollama:11434/api/pull" --json "{
        \"model\": \"${model_name}\"
    }"

    # Create a model
    curl -H "Authorization: Bearer ${API_KEY}" "http://open-webui/api/v1/models/create" -X POST --json "{
        \"id\": \"${model_name}\",
        \"name\": \"${model_name}\",
        \"meta\": {},
        \"params\": {},
        \"object\": \"model\",
        \"owned_by\": \"ollama\",
        \"ollama\": {
            \"name\": \"${model_name}\",
            \"model\": \"${model_name}\"
        },
        \"access_control\": ${access_control}
    }" || true

    # Change permission
    curl -H "Authorization: Bearer ${API_KEY}" "http://open-webui/api/v1/models/model/update?id=${model_name}" -X POST --json "{
        \"id\": \"${model_name}\",
        \"name\": \"${model_name}\",
        \"meta\": {},
        \"params\": {},
        \"object\": \"model\",
        \"owned_by\": \"ollama\",
        \"ollama\": {
            \"name\": \"${model_name}\",
            \"model\": \"${model_name}\"
        },
        \"access_control\": ${access_control}
    }"
}

# Provisioning all models
{{- range $_ := .Values.models.pull }}
{{- $name := .name }}
{{- $accessControlProfile := .accessControlProfile | default $.Values.models.defaultAccessControlProfile }}
{{- /* Validate Inputs */}}
{{- if empty $name }}
{{- fail "Empty model name has found" }}
{{- end }}
{{- if not ( has $accessControlProfile ( list "private" "public" ) ) }}
{{- fail ( printf "Unsupported access control profile: %s" $accessControlProfile ) }}
{{- end }}
provisioning_model \
    {{ $name | quote }} \
    {{ $accessControlProfile | quote }}
{{- end }}

# Finished!
exec true
