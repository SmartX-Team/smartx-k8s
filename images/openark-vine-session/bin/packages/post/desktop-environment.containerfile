# Add scripts
ADD ./config/weston.ini /etc/xdg/weston/weston.ini
ADD --chmod=0400 ./polkit/rules.d/* /etc/polkit-1/rules.d/
ADD --chmod=0555 ./bin/*.sh /opt/scripts/

# Add utilities
ENV ADDONS_HOME="/opt/openark/vdi/desktop"
ADD ./addons "${ADDONS_HOME}"

# Add users
USER {{ .Values.user.name | quote }}
WORKDIR {{ printf "/home/%s" .Values.user.name | quote }}
