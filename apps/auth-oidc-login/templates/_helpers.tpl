{{/*
Expand the name of the chart.
*/}}
{{- define "helm.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "helm.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "helm.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "helm.labels" -}}
helm.sh/chart: {{ include "helm.chart" . }}
{{ include "helm.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "helm.selectorLabels" -}}
app.kubernetes.io/name: {{ include "helm.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
OIDC name prefixes
*/}}
{{- define "helm.groupPrefix" -}}
{{- default "oidc:" }}
{{- end }}

{{/*
Viewer rules
*/}}
{{- define "helm.viewerRules" -}}
- apiGroups:
    - ""
  resources:
    - configmaps
    - endpoints
    - events
    - persistentvolumeclaims
    - pods
    - services
  verbs:
    - get
    - list
    - watch
- apiGroups:
    - ""
  resources:
    - pods/log
  verbs:
    - get
    - watch
- apiGroups:
    - apps
  resources:
    - deployments
    - replicasets
    - statefulsets
  verbs:
    - get
    - list
    - watch
- apiGroups:
    - argoproj.io
  resources:
    - cronworkflows
    - workflows
    - workflowtaskresults
    - workflowtemplates
  verbs:
    - get
    - list
    - watch
- apiGroups:
    - autoscaling
  resources:
    - horizontalpodautoscalers
  verbs:
    - get
    - list
    - watch
- apiGroups:
    - batch
  resources:
    - cronjobs
    - jobs
  verbs:
    - get
    - list
    - watch
- apiGroups:
    - discovery.k8s.io
  resources:
    - endpointslices
  verbs:
    - get
    - list
    - watch
- apiGroups:
    - gateway.networking.k8s.io
  resources:
    - gateways
  verbs:
    - get
    - list
    - watch
- apiGroups:
    - networking.k8s.io
  resources:
    - ingresses
  verbs:
    - get
    - list
    - watch
{{- if .Values.features.vine }}
- apiGroups:
    - org.ulagbulag.io
  resources:
    - catalogitems
    - tables
  verbs:
    - get
    - list
    - watch
{{- end }}
{{- end }}
