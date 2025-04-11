{{- define "podTemplate.ananicy" -}}
name: ananicy
image: "{{ .Values.ananicy.image.repo | default .Values.session.image.repo }}:{{ .Values.ananicy.image.tag | default .Values.session.image.tag | default .Chart.AppVersion }}"
imagePullPolicy: {{ .Values.ananicy.image.pullPolicy | default .Values.session.image.pullPolicy | quote }}
command:
  - /usr/bin/env
  - /usr/bin/ananicy
args:
  - start
env:
  - name: USER
    value: "0"
restartPolicy: Always
securityContext:
  privileged: true
  runAsNonRoot: false
  runAsUser: 0
workingDir: /
volumeMounts:

{{- /********************************/}}
  - name: cgroup
    mountPath: /sys/fs/cgroup
    readOnly: true

{{- /********************************/}}
{{- if .Values.features.devicePassthrough }}
  - name: dev
    mountPath: /dev
{{- end }}

{{- end }}
