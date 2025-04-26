{{- define "podTemplate.notebook" -}}
{{- include "helm.externalServiceContainerTemplate" ( merge ( dict
  "name" "notebook"
  "env" list
) . ) }}
command:
  - /usr/bin/env
  - jupyter
args:
  - notebook
ports:
  - name: notebook
    protocol: TCP
    containerPort: 8888
{{- end }}
