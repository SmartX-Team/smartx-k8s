{{- /*
Default IPMItool image
*/}}
{{- define "helm.ipmitool.image" -}}
{{- printf "%s:%s" .Values.ipmitool.image.repo ( .Values.ipmitool.image.tag | default .Chart.AppVersion ) }}
{{- end }}
