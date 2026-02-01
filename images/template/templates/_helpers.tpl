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
Mount the shared cache directory with given `key` and `target` pair.
*/}}
{{- define "container.run.cached" -}}
{{ include "container.run.cached.arg" . }}
RUN {{ printf "--mount=type=cache,sharing=locked,id=cache-%s-${TARGETOS}-${TARGETARCH}-${TARGETVARIANT}-%s,target=%s" .id .key ( .target | quote ) }}
{{- end }}

{{/*
Import builtin build arguments.
*/}}
{{- define "container.run.cached.arg" -}}
ARG TARGETARCH
ARG TARGETOS
ARG TARGETVARIANT
{{- end }}

{{/*
Mount the shared cache directory with given `key` and `target` pair on amd64 architecture.
*/}}
{{- define "container.run.cached.amd64" -}}
ARG TARGETOS
RUN {{ printf "--mount=type=cache,sharing=locked,id=cache-%s-${TARGETOS}-amd64--%s,target=%s" .id .key ( .target | quote ) }}
{{- end }}

{{/*
Mount the shared cache directory with given `key` and `target` pair without RUN prefix.
*/}}
{{- define "container.run.cached.bulked" -}}
{{- printf "--mount=type=cache,sharing=locked,id=cache-%s-${TARGETOS}-${TARGETARCH}-${TARGETVARIANT}-%s,target=%s" .id .key ( .target | quote ) }}
{{- end }}

{{/*
Mount the shared Rust cache directory.
*/}}
{{- define "container.run.cached.rust" -}}
{{- include "container.run.cached" ( dict
  "id" .Release.Name
  "key" "user"
  "target" "/root/.cache"
) }} {{ include "container.run.cached.bulked" ( dict
  "id" .Release.Name
  "key" "target"
  "target" "/src/target"
) }} {{ include "container.run.cached.bulked" ( dict
  "id" .Release.Name
  "key" "cargo-git"
  "target" "/usr/local/cargo/git"
) }} {{ include "container.run.cached.bulked" ( dict
  "id" .Release.Name
  "key" "cargo-registry"
  "target" "/usr/local/cargo/registry"
) }}
{{- end }}
