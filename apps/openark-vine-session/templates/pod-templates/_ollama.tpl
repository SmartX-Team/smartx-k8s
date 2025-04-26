{{- define "podTemplate.ollama" -}}
{{- include "helm.externalServiceContainerTemplate" ( merge ( dict
  "name" "ollama"
  "env" list
) . ) }}
ports:
  - name: ollama
    protocol: TCP
    containerPort: 11434
{{- end }}
