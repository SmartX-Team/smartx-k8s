{{- define "podTemplate.init-driver" -}}
name: init-driver
image: "{{ .Values.greeter.image.repo }}:{{ .Values.greeter.image.tag | default .Chart.AppVersion }}"
imagePullPolicy: {{ .Values.greeter.image.pullPolicy | quote }}
command:
  - env
  - bash
  - -c
args:
  - |
{{- .Files.Get "bin/init_driver.sh" | trim | nindent 4 }}
securityContext:
  capabilities:
    add:
      - SYS_ADMIN
  privileged: true
  runAsNonRoot: false
  runAsUser: 0
workingDir: /
volumeMounts:

{{- /********************************/}}
  - name: host-sys
    mountPath: /sys

{{- /********************************/}}
  - name: modules
    mountPath: /lib/modules

{{- end }}
