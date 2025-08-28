{{- /*
User ID
*/}}
{{- define "helm.userId" -}}
{{- if .Values.session.context.root }}
{{- int 0 }}
{{- else }}
{{- int .Values.session.context.uid }}
{{- end }}
{{- end }}

{{- /*
User home directory path
*/}}
{{- define "helm.userHome" -}}
{{- if .Values.session.context.root }}
{{- printf "/root" }}
{{- else }}
{{- printf "/home/user" }}
{{- end }}
{{- end }}

{{- /*
User home directory path (server-side)
*/}}
{{- define "helm.userHomeSubPath" -}}
{{- if eq .Values.volumes.home.type "LocalOwned" }}
{{- printf "home/%s" ( include "helm.userName" $ ) }}
{{- else if eq .Values.volumes.home.type "LocalShared" }}
{{- printf "home/_shared" }}
{{- else }} {{- /* Remote | Temporary */}}
{{- printf "home" }}
{{- end }}
{{- end }}

{{- /*
User containers directory path
*/}}
{{- define "helm.userContainersHome" -}}
{{- printf "/var/lib/containers" }}
{{- end }}

{{- /*
User containers directory path (server-side)
*/}}
{{- define "helm.userContainersHomeSubPath" -}}
{{- if eq .Values.volumes.home.type "LocalOwned" }}
{{- printf "containers/%s" ( include "helm.userName" $ ) }}
{{- else if eq .Values.volumes.home.type "LocalShared" }}
{{- printf "containers/_shared" }}
{{- else }} {{- /* Remote | Temporary */}}
{{- printf "containers" }}
{{- end }}
{{- end }}

{{- /*
User data directory path
*/}}
{{- define "helm.userDataHome" -}}
{{- printf "%s/.openark" ( include "helm.userHome" $ ) }}
{{- end }}

{{- /*
User data directory path (server-side)
*/}}
{{- define "helm.userDataHomeSubPath" -}}
{{- if eq .Values.volumes.home.type "LocalOwned" }}
{{- printf "data/%s/%s" ( .Values.mode | kebabcase ) ( include "helm.userName" $ ) }}
{{- else if eq .Values.volumes.home.type "LocalShared" }}
{{- printf "data/%s/_shared" ( .Values.mode | kebabcase ) }}
{{- else }} {{- /* Remote | Temporary */}}
{{- printf "data/%s" ( .Values.mode | kebabcase ) }}
{{- end }}
{{- end }}

{{- /*
User data directory path (server-side / bluetoothd)
*/}}
{{- define "helm.userDataBluetoothSubPath" -}}
{{- if eq .Values.volumes.home.type "LocalOwned" }}
{{- printf "data/%s/%s" "_bluetooth" ( include "helm.userName" $ ) }}
{{- else if eq .Values.volumes.home.type "LocalShared" }}
{{- printf "data/%s/_shared" "_bluetooth" }}
{{- else }} {{- /* Remote | Temporary */}}
{{- printf "data/%s" "_bluetooth" }}
{{- end }}
{{- end }}

{{- /*
User data directory path (server-side / sshd)
*/}}
{{- define "helm.userDataSshHomeSubPath" -}}
{{- if eq .Values.volumes.home.type "LocalOwned" }}
{{- printf "data/%s/%s" "_ssh" ( include "helm.userName" $ ) }}
{{- else if eq .Values.volumes.home.type "LocalShared" }}
{{- printf "data/%s/_shared" "_ssh" }}
{{- else }} {{- /* Remote | Temporary */}}
{{- printf "data/%s" "_ssh" }}
{{- end }}
{{- end }}

{{- /*
Wayland Backends
*/}}
{{- define "helm.waylandBackends" -}}
{{- $backends := list }}
{{- if .Values.services.rdp.enabled }}
{{- $backends := append $backends "rdp" }}
{{- end }}
{{- if .Values.services.x11vnc.enabled }}
{{- $backends := append $backends "vnc" }}
{{- end }}
{{- $backends | uniq | sortAlpha | join "," }}
{{- end }}

{{- /*
Pod affinity
*/}}
{{- define "helm.affinity" -}}
nodeAffinity:
  preferredDuringSchedulingIgnoredDuringExecution:
    - preference:
        matchExpressions:
          - key: node-role.kubernetes.io/kiss
            operator: In
            values:
              - Desktop
      weight: 1
  requiredDuringSchedulingIgnoredDuringExecution:
    nodeSelectorTerms:
{{- if not ( empty .Values.node.name ) }}
      - matchExpressions:
          - key: kubernetes.io/hostname
            operator: In
            values:
              - {{ .Values.node.name | quote }}
{{- else }}
      - matchExpressions:
          - key: node-role.kubernetes.io/kiss
            operator: In
            values:
              - Compute
              - Desktop
      - matchExpressions:
          - key: node-role.kubernetes.io/standalone
            operator: In
            values:
              - "true"
{{- end }}
{{- end }}

{{- /*
Pod metadata
*/}}
{{- define "helm.podMetadata" -}}
annotations:
  kubectl.kubernetes.io/default-container: {{ .Values.mode | kebabcase | quote }}
labels:
{{- include "helm.labels" $ | nindent 2 }}
  app.kubernetes.io/component: session
  {{ index .Values.openark.labels "org.ulagbulag.io/bind" | quote }}: "true"
{{- if not ( empty .Values.node.name ) }}
  {{ index .Values.openark.labels "org.ulagbulag.io/bind.node" | quote }}: {{ .Values.node.name | quote }}
{{- end }}
{{- if eq "Guest" .Values.user.kind }}
  {{ index .Values.openark.labels "org.ulagbulag.io/bind.user" | quote }}: ""
{{- else }}
  {{ index .Values.openark.labels "org.ulagbulag.io/bind.user" | quote }}: {{ .Values.user.name | quote }}
{{- end }}
{{- range $key, $value := .Values.features }}
  {{ printf "org.ulagbulag.io/feature-%s" ( $key | kebabcase ) }}: {{ $value | quote }}
{{- end }}
{{- end }}

{{- /*
External service container template
*/}}
{{- define "helm.externalServiceContainerTemplate" -}}

{{- range $_ := list "name" "env" }}
{{- if not ( hasKey $ . ) }}
{{- fail ( printf "Internal error: external service field not defined: %s" . ) }}
{{- end }}
{{- end }}

{{- range $_ := list "image" "imagePullPolicy"
  "command" "args" "ports" "resources" "securityContext"
  "volumeMounts" "workingDir"
}}
{{- if hasKey $ . }}
{{- fail ( printf "Internal error: external service field not supported: %s" . ) }}
{{- end }}
{{- end }}

{{- $service := index .Values.externalServices .name }}

name: {{ .name | kebabcase | quote }}
image: {{ printf "%s:%s"
  ( $service.image.repo | default .Values.session.image.repo )
  ( $service.image.tag | default .Values.session.image.tag | default .Chart.AppVersion )
}}
imagePullPolicy: {{
  $service.image.pullPolicy
  | default .Values.session.image.pullPolicy
  | quote
}}
env:
{{- include "podTemplate.desktop.env" $ | nindent 2 }}
{{- range $_ := .env | default list }}
  - {{- . | toYaml | nindent 4 }}
{{- end }}
{{- include "helm.podResources" $ }}
securityContext:
{{- include "podTemplate.desktop.securityContext" $ | nindent 2 }}
workingDir: {{ include "helm.userHome" $ | quote }}
volumeMounts:
{{- include "podTemplate.desktop.volumeMounts" $ | nindent 2 }}
{{- end }}

{{- /*
Pod resources
*/}}
{{- define "helm.podResources" -}}
{{- if not ( has $.Values.session.qos ( list "Burstable" "Guaranteed" ) ) }}
{{- fail "Unknown QoS Policy: %s" ( $.Values.session.qos | quote ) }}
{{- end }}
{{- if or
  $.Values.features.containers
  $.Values.volumes.public.enabled
  $.Values.volumes.static.enabled
}}
{{- $_ := set $.Values.session.resources "limits" ( $.Values.session.resources.limits | default dict ) }}
{{- $_ := set $.Values.session.resources.limits "squat.ai/fuse" "1" }}
{{- end }}
{{- if not ( empty $.Values.session.resources ) }}
resources:
{{- if $.Values.session.resources.claims }}
  claims:
{{- $.Values.session.resources.claims | toYaml | nindent 4 }}
{{- end }}
  limits:
{{- range $key, $value := $.Values.session.resources.limits | default dict }}
{{- if or
  ( and ( eq $.Values.session.qos "Burstable"  ) ( not ( has $key ( list "cpu" "memory" ) ) ) )
  ( and ( eq $.Values.session.qos "Guaranteed" ) )
}}
    {{ $key | quote }}: {{ $value | quote }}
{{- end }}
{{- end }}
{{- if ne $.Values.session.qos "Guaranteed" }}
  requests:
{{- range $key, $value := $.Values.session.resources.limits | default dict | merge ( $.Values.session.resources.requests | default dict ) }}
{{- if has $key ( list "cpu" "memory" ) }}
    {{ $key | quote }}: {{ $value | quote }}
{{- end }}
{{- end }}
{{- end }}
{{- end }}
{{- end }}

{{- /*
Pod init containers
*/}}
{{- define "helm.podInitContainers" -}}
initContainers:

{{- /********************************
    Ananicy
*************************************/}}
{{- if .Values.ananicy.enabled }}
  - {{- include "podTemplate.ananicy" $ | nindent 4 }}
{{- end }}

{{- /********************************
    Permission Checker
*************************************/}}
  - {{- include "podTemplate.init-check-permissions" $ | nindent 4 }}

{{- /********************************
    Driver Loader
*************************************/}}
{{- if .Values.features.hostDisplay }}
  - {{- include "podTemplate.init-driver" $ | nindent 4 }}
{{- end }}

{{- /********************************
    DBus Daemon
*************************************/}}
{{- if .Values.features.dbus }}
{{- if not .Values.features.hostDBus }}
  - {{- include "podTemplate.dbus-system" $ | nindent 4 }}
{{- end }}
{{- else if .Values.features.hostDBus }}
{{- fail "host DBus cannot be enabled without DBus" }}
{{- end }}

{{- /********************************
    Xorg Daemon
*************************************/}}
{{- if .Values.features.hostDisplay }}
{{- if not .Values.features.dbus }}
{{- fail "host display cannot be enabled without DBus" }}
{{- end }}
{{- if ne "Desktop" .Values.mode }}
{{- fail "host display cannot be enabled without desktop environment" }}
{{- end }}
  - {{- include "podTemplate.xorg" $ | nindent 4 }}
{{- end }}

{{- /********************************
    Bluetooth Daemon
*************************************/}}
{{- if .Values.features.hostBluetooth }}
{{- if not .Values.features.dbus }}
{{- fail "host bluetooth cannot be enabled without DBus" }}
{{- end }}
{{- if ne "Desktop" .Values.mode }}
{{- fail "host bluetooth cannot be enabled without desktop environment" }}
{{- end }}
  - {{- include "podTemplate.bluetoothd" $ | nindent 4 }}
{{- end }}

{{- /********************************
    PipeWire Audio Daemon
*************************************/}}
{{- if .Values.features.audio }}
{{- if not .Values.features.dbus }}
{{- fail "audio cannot be enabled without DBus" }}
{{- end }}
{{- if ne "Desktop" .Values.mode }}
{{- fail "audio cannot be enabled without desktop environment" }}
{{- else if not .Values.features.hostAudio }}
{{- fail ( print
  "Audio feature cannot be enabled without host audio"
  "\n* TODO: to be implemented"
) }}
{{- end }}
  - {{- include "podTemplate.pipewire" $ | nindent 4 }}
  - {{- include "podTemplate.wireplumber" $ | nindent 4 }}
  - {{- include "podTemplate.pipewire-pulse" $ | nindent 4 }}
{{- end }}

{{- /********************************
    Bluetooth Daemon
*************************************/}}
{{- if eq "Desktop" .Values.mode }}
{{- if eq "picom" .Values.session.compositor.x11 }}
  - {{- include "podTemplate.picom" $ | nindent 4 }}
{{- else }}
{{- fail "Unknown X11 compositor: %s" ( .Values.session.compositor.x11 | quote ) }}
{{- end }}
{{- end }}

{{- end }}

{{- /*
Pod containers
*/}}
{{- define "helm.podContainers" -}}
containers:

{{- /********************************
    Default Mode
*************************************/}}
{{- if eq "true" ( include "helm.serviceMode.isPod" .Values.mode ) }}
  - {{- include ( printf "podTemplate.%s" ( .Values.mode | kebabcase ) ) $
        | nindent 4
    }}
{{- end }}

{{- /********************************
    X11vnc Service
*************************************/}}
{{- if and .Values.services.x11vnc.enabled .Values.features.hostDisplay }}
  - {{- include "podTemplate.x11vnc" $ | nindent 4 }}
{{- end }}

{{- /********************************
    noVNC Service
*************************************/}}
{{- if .Values.services.novnc.enabled }}
  - {{- include "podTemplate.novnc" $ | nindent 4 }}
{{- end }}

{{- end }}

{{- /*
Pod template
*/}}
{{- define "helm.podTemplate" -}}

{{- /********************************/}}
affinity:
{{- include "helm.affinity" $ | nindent 2 }}

{{- /********************************/}}
initContainers:
{{- index ( include "helm.podInitContainers" $ | fromYaml ) "initContainers" | toYaml | nindent 2 }}

{{- /********************************/}}
containers:
{{- $containers := index ( include "helm.podContainers" $ | fromYaml ) "containers" }}
{{- if empty $containers }}
{{- fail "nothing to be deployed" }}
{{- end }}
{{- $containers | toYaml | nindent 2 }}

{{- /********************************
    DNS Context
*************************************/}}
dnsConfig:
  nameservers:
{{- .Values.dnsConfig.nameservers | default list | toYaml | nindent 6 }}
dnsPolicy: {{ .Values.dnsPolicy | default "Default" | quote }}

{{- /********************************
    Pod Context
*************************************/}}
hostIPC: {{ or .Values.session.context.hostIPC }}
hostNetwork: {{ .Values.session.context.hostNetwork }}
hostPID: {{ .Values.session.context.hostPID }}
hostname: "{{ include "helm.fullname" $ }}-{{ .Release.Namespace }}"
priorityClassName: {{ .Values.session.priorityClassName | quote }}
# TODO(HoKim98): Improve `PodLevelResources` feature gate (maybe co-work?)
# resources:
restartPolicy: {{ .Values.persistence.enabled | ternary "Always" "Never" | quote }}
securityContext:
  appArmorProfile:
    type: Unconfined
  seccompProfile:
    type: Unconfined
serviceAccount: {{ include "helm.serviceAccountName" $ | quote }}
shareProcessNamespace: {{ not .Values.session.context.hostPID }}  # TODO: To be removed!
terminationGracePeriodSeconds: 60
tolerations:
  - operator: Exists
    effect: NoSchedule
  - operator: Exists
    effect: NoExecute

{{- /********************************
    Volumes
*************************************/}}
volumes:
{{- include "helm.podVolumes" $ | nindent 2 }}
{{- end }}
