{{- define "podTemplate.bluetoothd" -}}
name: bluetoothd
image: "{{ .Values.greeter.image.repo }}:{{ .Values.greeter.image.tag | default .Chart.AppVersion }}"
imagePullPolicy: {{ .Values.greeter.image.pullPolicy | quote }}
command:
  - /usr/bin/env
  - bluetoothd
  - --experimental
  - --kernel
  - --nodetach
args: []
env:
  - name: DBUS_SYSTEM_BUS_ADDRESS
    value: "unix:path=/run/dbus/system_bus_socket"
restartPolicy: Always
securityContext:
  # FIXME: How to disable privileged permission?
  privileged: true # required to access to: /dev/snd (ALSA)
  runAsNonRoot: false
  runAsUser: 0
volumeMounts:

{{- /********************************/}}
  - name: home
    mountPath: /var/lib/bluetooth
    subPath: {{ include "helm.userDataBluetoothSubPath" $ | quote }}

{{- /********************************/}}
  - name: host-sys
    mountPath: /sys

{{- /********************************/}}
  - name: runtime-dbus
    mountPath: /run/dbus
    readOnly: true

{{- /********************************/}}
  - name: runtime-udev
    mountPath: /run/udev
    readOnly: true

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
