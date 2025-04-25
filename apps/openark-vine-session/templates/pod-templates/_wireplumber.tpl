{{- define "podTemplate.wireplumber" -}}
name: wireplumber
image: "{{ .Values.greeter.image.repo }}:{{ .Values.greeter.image.tag | default .Chart.AppVersion }}"
imagePullPolicy: {{ .Values.greeter.image.pullPolicy | quote }}
command:
  - /usr/bin/env
  - wireplumber
args: []
env:
  - name: DISABLE_RTKIT
    value: "y"
  - name: XDG_RUNTIME_DIR
    value: "/run/user/{{ include "helm.userId" $ }}"
readinessProbe:
  exec:
    command:
      - wpctl
      - status
  initialDelaySeconds: 1
  periodSeconds: 15
{{- /* Resources */}}
{{- if $.Values.session.resources.limits }}
resources:
  limits:
{{- range $key, $value := $.Values.session.resources.limits }}
{{- if has $key ( list "nvidia.com/gpu" ) }}
    {{ $key | quote }}: {{ $value | quote }}
{{- end }}
{{- end }}
{{- end }}
restartPolicy: Always
securityContext:
  # FIXME: How to disable privileged permission?
  # FIXME: Maybe related to: /proc/asound/cards
  privileged: true # required to access to: /dev/snd (ALSA)
  runAsNonRoot: {{ not ( .Values.session.context.root | default false ) }}
  runAsUser: {{ include "helm.userId" $ }}
volumeMounts:
  - name: dev-snd
    mountPath: /dev/snd
    readOnly: true
  - name: runtime-dbus
    mountPath: /run/dbus
    readOnly: true
  - name: runtime-udev
{{- if not .Values.features.hostUdev }}
{{- fail "Host audio cannot be enabled without host Udev" }}
{{- else }}
    mountPath: /run/udev
    readOnly: true
{{- end }}
  - name: runtime-user
    mountPath: "/run/user/{{ include "helm.userId" $ }}"
  - name: tmp
    mountPath: /tmp
  - name: tmp-ice
    mountPath: /tmp/.ICE-unix
    readOnly: true
  - name: tmp-x11
    mountPath: /tmp/.X11-unix
    readOnly: true
{{- end }}
