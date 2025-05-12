{{- define "podTemplate.nvidia-triton" -}}
{{- include "helm.externalServiceContainerTemplate" ( merge ( dict
  "name" "nvidiaTriton"
  "env" ( list
) ) . ) }}
command:
  - /usr/bin/env
  - tritonserver
args:
  - --model-control-mode=explicit
  - --model-repository={{ include "helm.userDataHome" $ }}
ports:
  - name: http
    protocol: TCP
    containerPort: 8000
  - name: grpc
    protocol: TCP
    containerPort: 8001
  - name: metrics
    protocol: TCP
    containerPort: 8002
{{- end }}
