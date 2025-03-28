{{- define "podTemplate.dbus-system" -}}
name: dbus-system
image: "{{ .Values.greeter.image.repo }}:{{ .Values.greeter.image.tag }}"
imagePullPolicy: {{ .Values.greeter.image.pullPolicy | quote }}
command:
  - /usr/bin/env
  - dbus-daemon
args:
  - --nofork
  - --system
livenessProbe:
  exec:
    command:
      - test
      - -S
      - /run/dbus/system_bus_socket
  initialDelaySeconds: 1
  periodSeconds: 5
# TODO: to be implemented
# readinessProbe: {}
restartPolicy: Always
securityContext:
  privileged: false
  runAsNonRoot: {{ not ( .Values.session.context.root | default false ) }}
  runAsUser: {{ include "helm.userId" $ }}
volumeMounts:
  - name: runtime-dbus
    mountPath: /run/dbus
{{- end }}
