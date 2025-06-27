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
{{- if .Values.session.context.root }}
{{- printf "/var/lib/containers" }}
{{- else }}
{{- printf "%s/.local/share/containers" ( include "helm.userHome" $ ) }}
{{- end }}
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
User SSH directory path (server-side)
*/}}
{{- define "helm.userSshHomeSubPath" -}}
{{- if eq .Values.volumes.home.type "LocalOwned" }}
{{- printf "ssh/%s" ( include "helm.userName" $ ) }}
{{- else if eq .Values.volumes.home.type "LocalShared" }}
{{- printf "ssh/_shared" }}
{{- else }} {{- /* Remote | Temporary */}}
{{- printf "ssh" }}
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
{{- /* Resources */}}
{{- if $.Values.session.resources.limits }}
resources:
  limits:
{{- range $key, $value := $.Values.session.resources.limits }}
{{- if not ( has $key ( list "cpu" "memory" ) ) }}
    # {{ $key | quote }}: {{ $value | quote }}
{{- end }}
    # TODO(HoKim98): Improve `PodLevelResources` feature gate (maybe co-work?)
    {{ $key | quote }}: {{ $value | quote }}
{{- end }}
{{- end }}
securityContext:
{{- include "podTemplate.desktop.securityContext" $ | nindent 2 }}
workingDir: {{ include "helm.userHome" $ | quote }}
volumeMounts:
{{- include "podTemplate.desktop.volumeMounts" $ | nindent 2 }}
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
    Pod Context
*************************************/}}
hostIPC: {{ .Values.session.context.hostIPC }}
hostNetwork: {{ .Values.session.context.hostNetwork }}
hostname: "{{ include "helm.fullname" $ }}-{{ .Release.Namespace }}"
priorityClassName: {{ .Values.session.priorityClassName | quote }}
# TODO(HoKim98): Improve `PodLevelResources` feature gate (maybe co-work?)
# resources:
{{- if $.Values.session.resources.claims }}
  # claims:
{{- $.Values.session.resources.claims | toYaml | nindent 4 }}
{{- end }}
{{- if $.Values.session.resources.limits }}
  # limits:
{{- range $key, $value := $.Values.session.resources.limits }}
{{- if has $key ( list "cpu" "memory" ) }}
    # {{ $key | quote }}: {{ $value | quote }}
{{- end }}
{{- end }}
{{- end }}
{{- if $.Values.session.resources.requests }}
  # requests:
{{- range $key, $value := $.Values.session.resources.requests }}
{{- if has $key ( list "cpu" "memory" ) }}
    # {{ $key | quote }}: {{ $value | quote }}
{{- end }}
{{- end }}
{{- end }}
restartPolicy: {{ .Values.persistence.enabled | ternary "Always" "Never" | quote }}
securityContext:
  appArmorProfile:
    type: Unconfined
  seccompProfile:
    type: Unconfined
serviceAccount: {{ include "helm.serviceAccountName" $ | quote }}
shareProcessNamespace: true  # TODO: To be removed!
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
