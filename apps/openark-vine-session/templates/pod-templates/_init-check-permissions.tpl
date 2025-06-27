{{- define "podTemplate.init-check-permissions" -}}
name: init-check-permissions
image: "{{ .Values.greeter.image.repo }}:{{ .Values.greeter.image.tag | default .Chart.AppVersion }}"
imagePullPolicy: {{ .Values.greeter.image.pullPolicy | quote }}
command:
  - env
  - bash
  - -c
args:
  - |
{{- tpl ( .Files.Get "bin/init_check_permissions.sh" ) . | trim | nindent 4 }}
env:
  - name: HOME
    value: {{ include "helm.userHome" $ | quote }}
  - name: TARGET_UID
    value: {{ include "helm.userId" $ | quote }}
resources:
  limits:
    cpu: 100m
    memory: 200Mi
securityContext:
  privileged: false
  runAsNonRoot: false
  runAsUser: 0
workingDir: /
volumeMounts:

{{- /********************************/}}
  - name: home
    mountPath: {{ include "helm.userHome" $ | quote }}
    subPath: {{ include "helm.userHomeSubPath" $ | quote }}

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
{{- if .Values.volumes.public.enabled }}
  - name: home-public
    mountPath: /mnt/public
{{- end }}

{{- /********************************/}}
  - name: host-sys
    mountPath: /host-sys

{{- /********************************/}}
  - name: runtime-user
    mountPath: "/run/user/{{ include "helm.userId" $ }}"

{{- /********************************/}}
{{- if eq "Desktop" .Values.mode }}
  - name: tmp-ice
    mountPath: /tmp/.ICE-unix
{{- end }}

{{- /********************************/}}
{{- if eq "Desktop" .Values.mode }}
  - name: tmp-x11
    mountPath: /tmp/.X11-unix
{{- end }}

{{- end }}
