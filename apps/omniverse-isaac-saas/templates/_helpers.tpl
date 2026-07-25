{{- define "omniverse-isaac-saas.name" -}}
{{- default .Chart.Name .Values.portal.name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- define "omniverse-isaac-saas.deploymentName" -}}
{{- default (include "omniverse-isaac-saas.name" .) .Values.portal.deploymentName | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- define "omniverse-isaac-saas.labels" -}}
app.kubernetes.io/name: {{ include "omniverse-isaac-saas.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end -}}
