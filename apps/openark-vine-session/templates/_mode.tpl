{{- define "helm.serviceMode.isPod" }}

{{- $nativeModes := list
  "Desktop"
  "Notebook"
  "NvidiaTriton"
  "Ollama"
}}

{{- $externalModes := list
  "Manual"
}}

{{- if not ( has . ( concat
  $nativeModes
  $externalModes
) ) }}
{{- fail ( printf "Unknown session mode: %s" . ) }}
{{- end }}

{{- has . $nativeModes }}

{{- end -}}
