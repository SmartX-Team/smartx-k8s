{{- define "podTemplate.novnc" -}}
name: novnc
image: "{{ .Values.services.novnc.image.repo }}:{{ .Values.services.novnc.image.tag }}"
imagePullPolicy: {{ .Values.services.novnc.image.pullPolicy | quote }}
env:
  - name: NOVNC_VNC_PATH
    value: /
ports:
  - name: novnc
    protocol: TCP
    containerPort: 6080
{{- end }}
