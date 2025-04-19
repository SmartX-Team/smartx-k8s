{{- /*
Agent name
*/}}
{{- define "helm.agentName" -}}
{{- printf "agent-%s"
  .agent.name
}}
{{- end }}

{{- /*
Agent labels
*/}}
{{- define "helm.agentLabels" -}}
h2pc.ulagbulag.io/kind: {{ .kind | kebabcase | quote }}
h2pc.ulagbulag.io/name: {{ .agent.name | quote }}
h2pc.ulagbulag.io/operator: {{ .name | quote }}
{{- end }}

{{- /*
Render an operator template
*/}}
{{- define "helm.agentOperatorRender" -}}

{{- /* Render the raw template */}}
{{- $rendered := tpl .template . | fromYaml }}
{{- with $rendered }}

{{- /* Validate the operator | Fields */}}
{{- range $field := list "kind" "params" "env" "features" "resources" "srcs" "sinks" "template" }}
{{- if not ( hasKey $rendered . ) }}
{{- fail ( printf "Missing operator field: %s.%s" $.name . ) }}
{{- end }}
{{- end }}

{{- /* Validate the operator | Parameters */}}
{{- range $expected := .params }}
{{- $_ := set $expected "exists" false }}
{{- range $key, $_ := $.params }}
{{- if eq $expected.name $key }}
{{- $_ := set $expected "exists" true }}
{{- end }}
{{- end }}

{{- /* Validate the operator | Parameters | Default */}}
{{- if not $expected.exists }}
{{- if hasKey $expected "default" }}
{{- $_ := set $.params $expected.name $expected.default }}
{{- else }}
{{- fail ( printf "Missing parameter: %s -> %s" $.name $expected.name ) }}
{{- end }}
{{- end }}

{{- /* Validate the operator | Parameters | Type */}}
{{- $given := index $.params $expected.name }}
{{- if eq "Boolean" $expected.type }}
{{- $_ := set $.params $expected.name ( has ( $given | toString | lower ) ( list "1" "true" ) ) }}
{{- else if eq "String" $expected.type }}
{{- $_ := set $.params $expected.name ( $given | toString ) }}
{{- else }}
{{- fail ( printf "Unknown parameter type: %s" $expected.type ) }}
{{- end }}

{{- end }}
{{- end }}

{{- /* Re-render the raw template */}}
{{- $rendered := tpl .template . | fromYaml }}
{{- with $rendered }}

{{- /* Format the output */}}
{{- . | toYaml }}
{{- end }}
{{- end }}

{{- /*
Agent operator
*/}}
{{- define "helm.agentOperator" -}}

{{- if not ( hasKey . "ExtraOperators" ) }}
{{- fail "Internal error: agent.ExtraOperators is not defined" }}
{{- end }}

{{- if not ( hasKey . "ExtraPrompts" ) }}
{{- fail "Internal error: agent.ExtraPrompts is not defined" }}
{{- end }}

{{- if not ( hasKey . "Files" ) }}
{{- fail "Internal error: agent.Files is not defined" }}
{{- end }}

{{- /* Validate inputs */}}
{{- if ne .operator ( .operator | title | nospace ) }}
{{- fail ( printf "Invalid agent operator name: %s should be PascalCase" ( .operator | quote ) ) }}
{{- end }}

{{- /* Guard operator template context */}}
{{ $context := dict
  "ExtraPrompts" .ExtraPrompts
  "Files" .Files
  "name" ( .operator | kebabcase )
  "params" ( .params | default list )
  "srcs" ( .srcs | default list )
  "template" nil
}}

{{- /* Load an operator template from extra operators */}}
{{- range $_ := .ExtraOperators }}
{{- if eq .name $context.name }}
{{- if empty $context.template }}
{{- $_ := set $context "template" .template }}
{{- else }}
{{- fail ( printf "Duplicated agent operator: %s" $.operator ) }}
{{- end }}
{{- end }}
{{- end }}

{{- /* Load an operator template from builtin operators */}}
{{- if empty $context.template }}
{{- $filePath := printf "operators/%s.yaml" $context.name }}
{{- if not ( empty ( $.Files.Glob $filePath ) ) }}
{{- $_ := set $context "template" ( $.Files.Get $filePath ) }}
{{- else }}
{{- fail ( printf "No such agent operator: %s" $.operator ) }}
{{- end }}
{{- end }}

{{- /* Render an operator template */}}
{{- $rendered := include "helm.agentOperatorRender" $context | fromYaml }}
{{- $_ := set $rendered "agent" . }}
{{- $_ := set $rendered "name" $context.name }}

{{- /* Format the output */}}
{{- $rendered | toYaml }}
{{- end }}
