{{- define "podTemplate.init-driver" -}}
name: init-driver
image: "{{ .Values.greeter.image.repo }}:{{ .Values.greeter.image.tag }}"
imagePullPolicy: {{ .Values.greeter.image.pullPolicy | quote }}
command:
  - env
  - bash
  - -c
args:
  - |
{{- .Files.Get "bin/init_driver.sh" | trim | nindent 4 }}
securityContext:
  privileged: true
  runAsNonRoot: false
  runAsUser: 0
workingDir: /
volumeMounts:

{{- /********************************/}}
  - name: modules
    mountPath: /lib/modules

{{- end }}
