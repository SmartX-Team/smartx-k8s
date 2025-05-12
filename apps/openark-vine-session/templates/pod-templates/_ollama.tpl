{{- define "podTemplate.ollama" -}}
{{- include "helm.externalServiceContainerTemplate" ( merge ( dict
  "name" "ollama"
  "env" ( list
    ( dict
      "name"  "OLLAMA_MODELS"
      "value" ( include "helm.userDataHome" $ )
    )
) ) . ) }}
ports:
  - name: ollama
    protocol: TCP
    containerPort: 11434
{{- end }}
