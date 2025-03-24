{{- define "helm.localPVName" -}}
{{- if empty .Values.node.name }}
{{- fail "Local storage cannot be enabled without node" }}
{{- end }}
{{- printf "%s-local-%s" ( include "helm.name" $ ) .Values.node.name }}
{{- end }}

{{- define "helm.localPVLabels" -}}
{{- include "helm.labels" $ }}
{{ index $.Values.openark.labels "org.ulagbulag.io/bind.node" | quote }}: {{ .Values.node.name | quote }}
{{- end }}

{{- define "helm.localPVPath" -}}
{{- printf "%s/%s" .Values.volumes.hostPathPrefix ( include "helm.name" $ ) }}
{{- end }}

{{- define "helm.localPVPath.vm.cdrom" -}}
{{- printf "%s/cdrom/%s" ( include "helm.localPVPath" $ ) .Values.vm.os }}
{{- end }}

{{- define "helm.localPVPath.vm.cdrom.scratch" -}}
{{- printf "%s/cdrom/%s/_scratch" ( include "helm.localPVPath" $ ) .Values.vm.os }}
{{- end }}

{{- define "helm.localPVPath.vm.shared" -}}
{{- printf "%s/vm/_shared/%s" ( include "helm.localPVPath" $ ) .Values.vm.os }}
{{- end }}

{{- define "helm.localPVNodeAffinity" -}}
required:
  nodeSelectorTerms:
{{- index ( include "helm.affinity" $ | fromYaml )
  "nodeAffinity"
  "requiredDuringSchedulingIgnoredDuringExecution"
  "nodeSelectorTerms" | toYaml | nindent 4
}}
{{- end }}

{{- define "helm.localPVCName" -}}
{{- printf "%s-local-%s" .Release.Name .Values.node.name }}
{{- end }}

{{- define "helm.remotePVCName" -}}
{{- printf "%s-remote-owned" ( include "helm.fullname" $ ) }}
{{- end }}
