{{- define "podTemplate.novnc" -}}
name: novnc
image: "{{ .Values.services.novnc.image.repo }}:{{ .Values.services.novnc.image.tag }}"
imagePullPolicy: {{ .Values.services.novnc.image.pullPolicy | quote }}
env:
  - name: NOVNC_VNC_PATH
{{- if empty .Values.node.name }}
    value: /
{{- else }}
    value: "/sessions/vnc/node/{{ .Values.node.name }}/"
{{- end }}
ports:
  - name: novnc
    protocol: TCP
    containerPort: 6080
resources:
  limits:
    cpu: "1"
    memory: 500Mi
{{- end }}
