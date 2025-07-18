{{- define "podTemplate.xorg" -}}
name: xorg
image: "{{ .Values.greeter.image.repo }}:{{ .Values.greeter.image.tag | default .Chart.AppVersion }}"
imagePullPolicy: {{ .Values.greeter.image.pullPolicy | quote }}
command:
  - /usr/bin/env
  - bash
args:
  - -c
  - |
{{- .Files.Get "bin/xorg.sh" | trim | nindent 4 }}
env:
  - name: DBUS_SYSTEM_BUS_ADDRESS
    value: "unix:path=/run/dbus/system_bus_socket"
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
  capabilities:
    add:
      - NET_ADMIN
      - SYS_ADMIN
  # FIXME: How to disable privileged permission?
  privileged: true # required to access to: /dev/input
  runAsNonRoot: {{ not ( .Values.session.context.root | default false ) }}
  runAsUser: {{ include "helm.userId" $ }}
volumeMounts:

{{- /********************************/}}
  - name: containerd-sock
    mountPath: /run/containerd/containerd.sock

{{- /********************************/}}
  - name: dev-input
    mountPath: /dev/input
    readOnly: true

{{- /********************************/}}
  - name: dev-snd
    mountPath: /dev/snd
    readOnly: true

{{- /********************************/}}
  - name: host-sys
    mountPath: /sys

{{- /********************************/}}
  - name: runtime-dbus
    mountPath: /run/dbus

{{- /********************************/}}
  - name: runtime-udev
{{- if not .Values.features.hostUdev }}
{{- fail "Host display cannot be enabled without host Udev" }}
{{- end }}
{{- if not .Values.session.context.hostNetwork }}
{{- /*
  host network is required to operate through PF_NETLINK sockets,
  which are used by udev monitor to receive kernel events
*/}}
{{- fail "Host Udev cannot be enabled without host network" }}
{{- end }}
    mountPath: /run/udev
    readOnly: true

{{- /********************************/}}
  - name: runtime-user
    mountPath: "/run/user/{{ include "helm.userId" $ }}"

{{- /********************************/}}
  - name: tmp-ice
    mountPath: /tmp/.ICE-unix

{{- /********************************/}}
  - name: tmp-x11
    mountPath: /tmp/.X11-unix

{{- end }}
