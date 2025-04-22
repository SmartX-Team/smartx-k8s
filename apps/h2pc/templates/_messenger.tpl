{{- /*
Messenger Bootstrapper Server
*/}}
{{- define "helm.messengerBootstrapperServer" -}}
{{- if eq "Kafka" $.Values.messenger.kind }}
{{- printf "%s-kafka-bootstrap.%s.svc"
  ( include "helm.fullname" $ )
  $.Release.Namespace
}}
{{- else if eq "Nats" $.Values.messenger.kind }}
{{- printf "%s-nats-headless.%s.svc"
  ( include "helm.fullname" $ )
  $.Release.Namespace
}}
{{- else }}
{{- fail ( printf "Unsupported messenger kind: %s" $.Values.messenger.kind ) }}
{{- end }}
{{- end }}

{{- /*
Messenger Bootstrapper Server URL
*/}}
{{- define "helm.messengerBootstrapperServerUrl" -}}
{{- if eq "Kafka" $.Values.messenger.kind }}
{{- printf "%s:9092" ( include "helm.messengerBootstrapperServer" $ ) }}
{{- else if eq "Nats" $.Values.messenger.kind }}
{{- printf "nats://%s:4222" ( include "helm.messengerBootstrapperServer" $ ) }}
{{- end }}
{{- end }}

{{- /*
Messenger Topics
*/}}
{{- define "helm.messengerTopics" -}}

{{- if not ( hasKey . "chartName" ) }}
{{- fail "Internal error: messenger.chartName is not defined" }}
{{- end }}

{{- if not ( hasKey . "operator" ) }}
{{- fail "Internal error: messenger.operator is not defined" }}
{{- end }}

{{- if not .topics }}
{{- fail "Internal error: messenger.topics is not defined or empty" }}
{{- end }}

{{- $context := dict
  "topics" list
}}

{{- range $_ := .topics }}
{{- if not ( has .kind ( list "Stream" ) ) }}
{{- fail ( printf "Unsupported topic kind: %s" .kind ) }}
{{- end }}

{{- $_ := set $context "topics" ( append $context.topics ( printf "%s.agent.%s.%s"
  $.chartName
  ( .name | default $.operator.agent.name )
  ( .kind | kebabcase )
) ) }}
{{- end }}

{{- /* Render the output */}}
{{- $context.topics | uniq | sortAlpha | join "," -}}
{{- end }}
