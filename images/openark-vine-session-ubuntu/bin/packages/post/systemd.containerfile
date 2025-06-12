USER {{ .Values.user.name | quote }}
WORKDIR {{ printf "/home/%s" .Values.user.name | quote }}
