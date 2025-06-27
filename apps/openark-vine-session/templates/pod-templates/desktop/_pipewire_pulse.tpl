{{- define "podTemplate.pipewire-pulse" -}}
name: pipewire-pulse
image: "{{ .Values.greeter.image.repo }}:{{ .Values.greeter.image.tag | default .Chart.AppVersion }}"
imagePullPolicy: {{ .Values.greeter.image.pullPolicy | quote }}
command:
  - /usr/bin/env
  - pipewire-pulse
args: []
env:
  - name: DISABLE_RTKIT
    value: "y"
  - name: XDG_RUNTIME_DIR
    value: "/run/user/{{ include "helm.userId" $ }}"
livenessProbe:
  exec:
    command:
      - test
      - -S
      - {{ printf "/run/user/%d/pulse/native" ( .Values.session.context.uid | int ) | quote }}
  initialDelaySeconds: 1
  periodSeconds: 5
readinessProbe:
  exec:
    command:
      - pactl
      - info
  initialDelaySeconds: 1
  periodSeconds: 15
restartPolicy: Always
securityContext:
  privileged: false
  runAsNonRoot: {{ not ( .Values.session.context.root | default false ) }}
  runAsUser: {{ include "helm.userId" $ }}
volumeMounts:

{{- /********************************/}}
  - name: runtime-dbus
    mountPath: /run/dbus
    readOnly: true

{{- /********************************/}}
  - name: runtime-user
    mountPath: "/run/user/{{ include "helm.userId" $ }}"

{{- /********************************/}}
  - name: tmp
    mountPath: /tmp

{{- end }}
