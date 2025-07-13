{{- define "podTemplate.picom" -}}
name: picom
image: "{{ .Values.greeter.image.repo }}:{{ .Values.greeter.image.tag | default .Chart.AppVersion }}"
imagePullPolicy: {{ .Values.greeter.image.pullPolicy | quote }}
command:
  - /usr/bin/env
  - picom
args:
{{- if .Values.features.dbus }}
  - --dbus
{{- end }}
{{- if .Values.session.compositor.options.vsync }}
  - --vsync
{{- else }}
  - --no-vsync
{{- end }}
env:
  - name: DBUS_SYSTEM_BUS_ADDRESS
    value: "unix:path=/run/dbus/system_bus_socket"
restartPolicy: Always
securityContext:
  runAsNonRoot: {{ not ( .Values.session.context.root | default false ) }}
  runAsUser: {{ include "helm.userId" $ }}
volumeMounts:

{{- /********************************/}}
{{- if .Values.features.dbus }}
  - name: runtime-user
    mountPath: "/run/user/{{ include "helm.userId" $ }}"
{{- end }}

{{- /********************************/}}
  - name: tmp
    mountPath: /tmp

{{- /********************************/}}
  - name: tmp-ice
    mountPath: /tmp/.ICE-unix
    readOnly: true

{{- /********************************/}}
  - name: tmp-x11
    mountPath: /tmp/.X11-unix
    readOnly: true

{{- end }}
