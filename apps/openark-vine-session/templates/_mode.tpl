{{- define "helm.serviceMode.isPod" }}

{{- $nativeModes := list
  "Desktop"
  "Notebook"
  "Ollama"
}}

{{- $externalModes := list
  "None"
}}

{{- if not ( has . ( concat
  $nativeModes
  $externalModes
) ) }}
{{- fail ( printf "Unknown session mode: %s" . ) }}
{{- end }}

{{- has . $nativeModes }}

{{- end -}}
