{{- /*
Kueue cluster name
*/}}
{{- define "helm.kueueClusterNamePrefix" -}}
{{- printf "openark.%s.%s" $.Release.Namespace ( include "helm.fullname" $ ) }}
{{- end }}
