{{- define "podTemplate.xorg" -}}
name: xorg
image: "{{ .Values.greeter.image.repo }}:{{ .Values.greeter.image.tag }}"
imagePullPolicy: {{ .Values.greeter.image.pullPolicy | quote }}
command:
  - /usr/bin/env
  - bash
args:
  - -c
  - |
{{- .Files.Get "bin/xorg.sh" | trim | nindent 4 }}
env:
  - name: DISPLAY
    value: ":0"
  - name: NVIDIA_DRIVER_CAPABILITIES
    value: display,graphics,utility,video
  - name: XDG_RUNTIME_DIR
    value: "/run/user/{{ include "helm.userId" $ }}"
livenessProbe:
  exec:
    command:
      - test
      - -S
      - /tmp/.X11-unix/X0
  initialDelaySeconds: 1
  periodSeconds: 5
readinessProbe:
  exec:
    command:
      - xrandr
      - --listactivemonitors
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
  privileged: true # required to access to: /dev/input
  runAsNonRoot: {{ not ( .Values.session.context.root | default false ) }}
  runAsUser: {{ include "helm.userId" $ }}
volumeMounts:
  - name: containerd-sock
    mountPath: /run/containerd/containerd.sock
  - name: dev-input
    mountPath: /dev/input
    readOnly: true
  - name: dev-snd
    mountPath: /dev/snd
    readOnly: true
  - name: runtime-dbus
    mountPath: /run/dbus
  - name: runtime-udev
{{- if not .Values.features.hostUdev }}
{{- fail "Host display cannot be enabled without host Udev" }}
{{- else }}
    mountPath: /run/udev
    readOnly: true
{{- end }}
  - name: runtime-user
    mountPath: "/run/user/{{ include "helm.userId" $ }}"
  - name: tmp-ice
    mountPath: /tmp/.ICE-unix
  - name: tmp-x11
    mountPath: /tmp/.X11-unix
{{- end }}
