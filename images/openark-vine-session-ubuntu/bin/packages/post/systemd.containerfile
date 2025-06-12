ADD {{ printf "./etc/systemd/%s.service" .Release.Name | quote }} {{ printf "/usr/lib/systemd/user/%s.service" .Release.Name | quote }}
