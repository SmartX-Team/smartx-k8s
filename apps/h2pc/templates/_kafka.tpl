{{- /*
Kafka Bootstrapper Server
*/}}
{{- define "helm.kafkaBootstrapperServer" -}}
{{- printf "%s-kafka-bootstrap" ( include "helm.fullname" $ ) }}
{{- end }}

{{- /*
Kafka Topics
*/}}
{{- define "helm.kafkaTopics" -}}

{{- if not ( hasKey . "chartName" ) }}
{{- fail "Internal error: kafka.chartName is not defined" }}
{{- end }}

{{- if not ( hasKey . "operator" ) }}
{{- fail "Internal error: kafka.operator is not defined" }}
{{- end }}

{{- if not .topics }}
{{- fail "Internal error: kafka.topics is not defined or empty" }}
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
  $.operator.agent.name
  ( .kind | kebabcase )
) ) }}
{{- end }}

{{- /* Render the output */}}
{{- $context.topics | uniq | sortAlpha | join "," -}}
{{- end }}
