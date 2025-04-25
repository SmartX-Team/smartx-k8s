{{- /*
Default wsmancli image
*/}}
{{- define "helm.wsmancli.image" -}}
{{- printf "%s:%s" .Values.wsmancli.image.repo ( .Values.wsmancli.image.tag | default .Chart.AppVersion ) }}
{{- end }}
