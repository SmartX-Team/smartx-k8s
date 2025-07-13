
{{- /********************************
    Session Environment Variables
*************************************/}}
{{- define "podTemplate.desktop.env" -}}
{{- range $env := .Values.session.env | default list }}
-
{{- . | toYaml | nindent 8 }}
{{- end }}
- name: DBUS_SYSTEM_BUS_ADDRESS
  value: "unix:path=/run/dbus/system_bus_socket"
- name: DISPLAY
  value: ":0"
- name: HOME
  value: {{ include "helm.userHome" $ | quote }}
- name: KISS_DESKTOP_FONTS_URL
  value: {{ .Values.session.template.fontsUrl | quote }}
- name: KISS_DESKTOP_ICONS_URL
  value: {{ .Values.session.template.iconsUrl | quote }}
- name: KISS_DESKTOP_THEMES_URL
  value: {{ .Values.session.template.themesUrl | quote }}
- name: KISS_DESKTOP_TEMPLATE_GIT
  value: {{ .Values.session.template.git | quote }}
- name: KISS_DESKTOP_TEMPLATE_GIT_BRANCH
  value: {{ .Values.session.template.gitBranch | quote }}
- name: LANG
  value: {{ .Values.session.locale.lang | default .Values.session.locale.global | quote }}
- name: LC_ALL
  value: {{ .Values.session.locale.lc.all | default .Values.session.locale.global | quote }}
- name: LOCALE
  value: {{ .Values.session.locale.global | quote }}
- name: NVIDIA_DRIVER_CAPABILITIES
  value: all
{{- if .Values.region.timezone }}
- name: TZ
  value: {{ .Values.region.timezone | quote }}
{{- end }}
- name: USER
  value: {{ include "helm.userId" $ | quote }}
- name: USER_SHELL
  value: "/bin/{{ .Values.user.shell }}"
- name: XDG_RUNTIME_DIR
  value: "/run/user/{{ include "helm.userId" $ }}"
- name: WAYLAND_BACKEND
  value: {{ include "helm.waylandBackends" $ | quote }}
{{- end }}

{{- /********************************
    Session Ports
*************************************/}}
{{- define "podTemplate.desktop.ports" -}}

{{- /********************************/}}
{{- if .Values.services.ssh.enabled }}
- name: ssh
  protocol: TCP
  containerPort: 22
{{- end }}

{{- /********************************/}}
{{- if and .Values.services.x11vnc.enabled ( not .Values.features.hostDisplay ) }}
- name: x11vnc
  protocol: TCP
  containerPort: 5900
{{- end }}

{{- /********************************/}}
{{- if and .Values.services.rdp.enabled ( not .Values.features.hostDisplay ) }}
- name: rdp-tcp
  protocol: TCP
  containerPort: 3389
- name: rdp-udp
  protocol: UDP
  containerPort: 3389
{{- end }}

{{- /********************************/}}
{{- range $port := .Values.extraServices }}
-
{{- if not ( empty .name ) }}
  name: {{ .name | quote }}
{{- end }}
{{- if not ( empty ( .containerPort | default .port ) ) }}
  containerPort: {{ .containerPort | default .port }}
{{- end }}
{{- if not ( empty .protocol ) }}
  protocol: {{ .protocol | quote }}
{{- end }}
{{- end }}

{{- end }}

{{- /********************************
    Session Security Context
*************************************/}}
{{- define "podTemplate.desktop.securityContext" -}}
{{- if .Values.session.context.privileged }}
capabilities:
  add:
    - SYS_ADMIN
{{- end }}
privileged: {{ .Values.session.context.privileged }}
runAsNonRoot: {{ not ( .Values.session.context.root | default .Values.session.context.sudo ) }}
runAsUser: {{ include "helm.userId" $ }}
{{- end }}

{{- /********************************/}}
{{- define "podTemplate.desktop.volumeMounts" -}}
{{- if .Values.features.devicePassthrough }}
- name: dev
  mountPath: /dev
{{- end }}

{{- /********************************/}}
{{- if .Values.features.hostDisplay }}
- name: dev-input
  mountPath: /dev/input
{{- end }}

{{- /********************************/}}
{{- if and ( not .Values.session.context.hostIPC ) .Values.features.ipcPassthrough }}
- name: dev-shm
  mountPath: /dev/shm
{{- end }}

{{- /********************************/}}
{{- if .Values.features.hostAudio }}
- name: dev-snd
  mountPath: /dev/snd
{{- end }}

{{- /********************************/}}
- name: home
  mountPath: {{ include "helm.userHome" $ | quote }}
  subPath: {{ include "helm.userHomeSubPath" $ | quote }}

{{- /********************************/}}
{{- if .Values.services.ssh.enabled }}
- name: home
  mountPath: /etc/ssh
  subPath: {{ include "helm.userDataSshHomeSubPath" $ | quote }}
{{- end }}

{{- /********************************/}}
{{- if .Values.features.containers }}
- name: home
  mountPath: {{ include "helm.userContainersHome" $ | quote }}
  subPath: {{ include "helm.userContainersHomeSubPath" $ | quote }}
{{- end }}

{{- /********************************/}}
{{- if .Values.features.data }}
- name: home
  mountPath: {{ include "helm.userDataHome" $ | quote }}
  subPath: {{ include "helm.userDataHomeSubPath" $ | quote }}
{{- end }}

{{- /********************************/}}
{{- if eq "Notebook" .Values.mode }}
- name: home-notebook
  mountPath: "{{ include "helm.userHome" $ }}/.jupyter/jupyter_notebook_config.py"
  subPath: "jupyter_notebook_config.py"
{{- end }}

{{- /********************************/}}
{{- if .Values.volumes.public.enabled }}
- name: home-public
  mountPath: /mnt/public
{{- end }}

{{- /********************************/}}
{{- if .Values.volumes.static.enabled }}
- name: home-static
  mountPath: /mnt/static
  readOnly: true
{{- end }}

{{- /********************************/}}
- name: host-sys
  mountPath: /sys/bus
  subPath: bus
  readOnly: true

{{- /********************************/}}
- name: machine-id
  mountPath: /etc/machine-id
  subPath: machine-id
  readOnly: true

{{- /********************************/}}
- name: logs
  mountPath: /var/log/journal

{{- /********************************/}}
{{- if .Values.features.dbus }}
- name: runtime-dbus
  mountPath: /run/dbus
  readOnly: true
{{- end }}

{{- /********************************/}}
- name: runtime-user
  mountPath: "/run/user/{{ include "helm.userId" $ }}"

{{- /********************************/}}
- name: tmp
  mountPath: /tmp

{{- /********************************/}}
{{- if eq "Desktop" .Values.mode }}
- name: tmp-ice
  mountPath: /tmp/.ICE-unix
  readOnly: true
{{- end }}

{{- /********************************/}}
{{- if eq "Desktop" .Values.mode }}
- name: tmp-x11
  mountPath: /tmp/.X11-unix
  readOnly: true
{{- end }}

{{- end }}

{{- /********************************/}}
{{- define "podTemplate.desktop" -}}
name: desktop
image: "{{ .Values.session.image.repo }}:{{ .Values.session.image.tag | default .Chart.AppVersion }}"
imagePullPolicy: {{ .Values.session.image.pullPolicy | quote }}

{{- if not ( empty .Values.session.command ) }}
command:
{{- .Values.session.command | toYaml | nindent 6 }}
{{- end }}

{{- if not ( empty .Values.session.args ) }}
args:
{{- .Values.session.args | toYaml | nindent 6 }}
{{- end }}

env:
{{- include "podTemplate.desktop.env" $ | nindent 2 }}
ports:
{{- include "podTemplate.desktop.ports" $ | nindent 2 }}
{{- include "helm.podResources" $ }}
securityContext:
{{- include "podTemplate.desktop.securityContext" $ | nindent 2 }}
workingDir: {{ include "helm.userHome" $ | quote }}
volumeMounts:
{{- include "podTemplate.desktop.volumeMounts" $ | nindent 2 }}
{{- end }}
