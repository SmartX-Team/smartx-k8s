{{- define "helm.podVolumes" -}}

{{- /********************************/}}
  - name: cgroup
{{- if .Values.ananicy.enabled }}
    emptyDir: null
    hostPath:
      path: /sys/fs/cgroup
      type: Directory
{{- else }}
    emptyDir: {}
    hostPath: null
{{- end }}

{{- /********************************/}}
{{- if .Values.features.hostDisplay }}
  - name: containerd-sock
    hostPath:
      path: /run/containerd/containerd.sock
      type: Socket
{{- end }}

{{- /********************************/}}
{{- if or .Values.features.devicePassthrough .Values.features.hostDisplay }}
  - name: dev
    hostPath:
      path: /dev
      type: Directory
{{- end }}

{{- /********************************/}}
  - name: dev-input
{{- if .Values.features.hostDisplay }}
    emptyDir: null
    hostPath:
      path: /dev/input
      type: Directory
{{- else }}
    emptyDir: {}
    hostPath: null
{{- end }}

{{- /********************************/}}
{{- if and ( not .Values.session.context.hostIPC ) .Values.features.ipcPassthrough }}
  - name: dev-shm
    emptyDir:
      medium: Memory
{{- end }}

{{- /********************************/}}
  - name: dev-snd
{{- if .Values.features.hostAudio }}
    emptyDir: null
    hostPath:
      path: /dev/snd
      type: Directory
{{- else }}
    emptyDir: {}
    hostPath: null
{{- end }}

{{- /********************************/}}
  - name: home
{{- if or
( eq .Values.volumes.home.type "LocalOwned" )
( eq .Values.volumes.home.type "LocalShared" )
}}
    emptyDir: null
    hostPath: null
    persistentVolumeClaim:
      claimName: {{ include "helm.localPVCName" $ | quote }}
{{- else if eq .Values.volumes.home.type "RemoteOwned" }}
    emptyDir: null
    hostPath: null
    persistentVolumeClaim:
      claimName: {{ include "helm.remotePVCName" $ | quote }}
{{- else if eq .Values.volumes.home.type "Temporary" }}
    emptyDir: {}
    hostPath: null
    persistentVolumeClaim: null
{{- else }}
{{- fail ( printf "Unknown home volume type: %s" .Values.volumes.home.type ) }}
{{- end }}

{{- /********************************/}}
{{- if eq "Notebook" .Values.mode }}
  - name: home-notebook
    secret:
      secretName: {{ include "helm.notebookName" $ | quote }}
      defaultMode: 292 # 0o444
{{- end }}

{{- /********************************/}}
  - name: home-public
{{- if .Values.volumes.public.enabled }}
{{- if empty .Values.volumes.public.persistentVolumeClaim.claimName }}
{{- fail "public volume cannot be enabled without manual claimName" }}
{{- end }}
    emptyDir: null
    persistentVolumeClaim:
      claimName: {{ tpl .Values.volumes.public.persistentVolumeClaim.claimName $ | quote }}
{{- else }}
    emptyDir: {}
    persistentVolumeClaim: null
{{- end }}

{{- /********************************/}}
  - name: home-static
{{- if .Values.volumes.static.enabled }}
{{- if empty .Values.volumes.static.persistentVolumeClaim.claimName }}
{{- fail "public volume cannot be enabled without manual claimName" }}
{{- end }}
    emptyDir: null
    persistentVolumeClaim:
      claimName: {{ tpl .Values.volumes.static.persistentVolumeClaim.claimName $ | quote }}
{{- else }}
    emptyDir: {}
    persistentVolumeClaim: null
{{- end }}

{{- /********************************/}}
  - name: host-sys
    hostPath:
      path: /sys
      type: Directory

{{- /********************************/}}
  - name: machine-id
    configMap:
      defaultMode: 365 # 0o555
      name: {{ include "helm.fullname" $ | quote }}
      items:
        - key: machine-id
          path: machine-id

{{- /********************************/}}
  - name: modules
{{- if .Values.features.hostDisplay }}
    emptyDir: null
    hostPath:
      path: /lib/modules
      type: Directory
{{- else }}
    emptyDir: {}
    hostPath: null
{{- end }}

{{- /********************************/}}
  - name: logs
    emptyDir: {}

{{- /********************************/}}
  - name: runtime-dbus
{{- if .Values.features.hostDBus }}
    emptyDir: null
    hostPath:
      path: /run/dbus
      type: Directory
{{- else }}
    emptyDir: {}
    hostPath: null
{{- end }}

{{- /********************************/}}
  - name: runtime-udev
{{- if .Values.features.hostUdev }}
    emptyDir: null
    hostPath:
      path: /run/udev
      type: Directory
{{- else }}
    emptyDir: {}
    hostPath: null
{{- end }}

{{- /********************************/}}
  - name: runtime-user
{{- if .Values.features.hostDBus }}
    emptyDir: null
    hostPath:
      path: "/run/user/{{ include "helm.userId" $ }}"
      type: DirectoryOrCreate
{{- else }}
    emptyDir: {}
    hostPath: null
{{- end }}

{{- /********************************/}}
  - name: tmp
    emptyDir: {}

{{- if eq "Desktop" .Values.mode }}
  - name: tmp-ice
    emptyDir: {}
{{- end }}

{{- if eq "Desktop" .Values.mode }}
  - name: tmp-x11
    emptyDir: {}
{{- end }}

{{- end }}
