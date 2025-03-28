{{- define "podTemplate.pipewire" -}}
name: pipewire
image: "{{ .Values.greeter.image.repo }}:{{ .Values.greeter.image.tag }}"
imagePullPolicy: {{ .Values.greeter.image.pullPolicy | quote }}
command:
  - /usr/bin/env
  - pipewire
args: []
env:
  - name: DISABLE_RTKIT
    value: "y"
  - name: DISPLAY
    value: ":0"
  - name: XDG_RUNTIME_DIR
    value: "/run/user/{{ include "helm.userId" $ }}"
livenessProbe:
  exec:
    command:
      - test
      - -S
      - /run/user/2000/pipewire-0
  initialDelaySeconds: 1
  periodSeconds: 5
restartPolicy: Always
securityContext:
  # FIXME: How to disable privileged permission?
  # FIXME: Maybe related to: /proc/asound/cards
  privileged: true
  runAsNonRoot: {{ not ( .Values.session.context.root | default false ) }}
  runAsUser: {{ include "helm.userId" $ }}
volumeMounts:

{{- /********************************/}}
  - name: dev-snd
    mountPath: /dev/snd
    readOnly: true

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

{{- /********************************/}}
  - name: tmp-ice
    mountPath: /tmp/.ICE-unix
    readOnly: true

{{- /********************************/}}
  - name: tmp-x11
    mountPath: /tmp/.X11-unix
    readOnly: true

{{- end }}
