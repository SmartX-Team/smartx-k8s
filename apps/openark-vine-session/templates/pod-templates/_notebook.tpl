{{- define "podTemplate.notebook" -}}
name: notebook
image: "{{ .Values.services.notebook.image.repo | default .Values.session.image.repo }}:{{ .Values.services.notebook.image.tag | default .Values.session.image.tag | default .Chart.AppVersion }}"
imagePullPolicy: {{ .Values.services.notebook.image.pullPolicy | default .Values.session.image.pullPolicy | quote }}
command:
  - /usr/bin/env
  - jupyter
args:
  - notebook
env:
{{- include "podTemplate.session.env" $ | nindent 2 }}
ports:
  - name: notebook
    protocol: TCP
    containerPort: 8888
{{- /* Resources */}}
{{- if $.Values.session.resources.limits }}
resources:
  limits:
{{- range $key, $value := $.Values.session.resources.limits }}
{{- if not ( has $key ( list "cpu" "memory" ) ) }}
    {{ $key | quote }}: {{ $value | quote }}
{{- end }}
{{- end }}
{{- end }}
securityContext:
{{- include "podTemplate.session.securityContext" $ | nindent 2 }}
workingDir: {{ include "helm.userHome" $ | quote }}
volumeMounts:
{{- include "podTemplate.session.volumeMounts" $ | nindent 2 }}
{{- end }}
