{{- define "podTemplate.x11vnc" -}}
name: x11vnc
image: "{{ .Values.services.x11vnc.image.repo }}:{{ .Values.services.x11vnc.image.tag | default .Chart.AppVersion }}"
imagePullPolicy: {{ .Values.services.x11vnc.image.pullPolicy | quote }}
env:
  - name: DISPLAY
    value: ":0"
  - name: X11VNC_ARGS
    value: -cursor most -noscr -nowcr -nowf -noxdamage
  - name: X11VNC_MULTIPTR
    value: "false"
  - name: X11VNC_XKB
    value: "true"
ports:
  - name: x11vnc
    protocol: TCP
    containerPort: 5900
securityContext:
  runAsNonRoot: {{ not ( .Values.session.context.root | default .Values.session.context.sudo ) }}
  runAsUser: {{ include "helm.userId" $ }}
volumeMounts:

{{- /********************************/}}
  - name: tmp-ice
    mountPath: /tmp/.ICE-unix
    readOnly: true

{{- /********************************/}}
  - name: tmp-x11
    mountPath: /tmp/.X11-unix
    readOnly: true

{{- end }}
