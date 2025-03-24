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
{{- printf "%s/.local/share/containers" ( include "helm.userHome" $ ) }}
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
      - matchExpressions:
{{- if not ( empty .Values.node.name ) }}
          - key: kubernetes.io/hostname
            operator: In
            values:
              - {{ .Values.node.name | quote }}
{{- else }}
          - key: node-role.kubernetes.io/kiss
            operator: In
            values:
              - Compute
              - Desktop
{{- end }}
{{- end }}

{{- /*
Pod metadata
*/}}
{{- define "helm.podMetadata" -}}
labels:
{{- include "helm.labels" $ | nindent 2 }}
  app.kubernetes.io/component: session
{{- if not ( empty .Values.node.name ) }}
  {{ index .Values.openark.labels "org.ulagbulag.io/bind.node" | quote }}: {{ .Values.node.name | quote }}
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
    DBus Daemon
*************************************/}}
{{- if not .Values.features.hostDBus }}
  - {{- include "podTemplate.dbus-system" $ | nindent 4 }}
{{- end }}

{{- /********************************
    Xorg Daemon
*************************************/}}
{{- if .Values.features.hostDisplay }}
{{- if not .Values.features.desktopEnvironment }}
{{- fail "host display cannot be enabled without desktop environment" }}
{{- end }}
  - {{- include "podTemplate.xorg" $ | nindent 4 }}
{{- end }}

{{- /********************************
    PipeWire Audio Daemon
*************************************/}}
{{- if .Values.features.audio }}
{{- if not .Values.features.desktopEnvironment }}
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
    User Session
*************************************/}}
{{- if .Values.features.desktopEnvironment }}
  - {{- include "podTemplate.session" $ | nindent 4 }}
{{- end }}

{{- /********************************
    Notebook Service
*************************************/}}
{{- if and .Values.services.notebook.enabled }}
{{- if .Values.features.desktopEnvironment }}
{{- fail "notebook service cannot be enabled with desktop environment" }}
{{- end }}
  - {{- include "podTemplate.notebook" $ | nindent 4 }}
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
restartPolicy: {{ .Values.persistence.enabled | ternary "Always" "Never" | quote }}
priorityClassName: {{ .Values.session.priorityClassName | quote }}
serviceAccount: {{ include "helm.serviceAccountName" $ | quote }}
shareProcessNamespace: true
terminationGracePeriodSeconds: 60
tolerations:
  - operator: Exists
    effect: NoExecute
  - operator: Exists
    effect: NoSchedule

{{- /********************************
    Volumes
*************************************/}}
volumes:
{{- include "helm.podVolumes" $ | nindent 2 }}
{{- end }}
