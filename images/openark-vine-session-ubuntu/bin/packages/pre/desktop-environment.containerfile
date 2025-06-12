# Add scripts
ADD ./config/weston.ini /etc/xdg/weston/weston.ini
ADD --chmod=0400 ./polkit/rules.d/* /etc/polkit-1/rules.d/
ADD --chmod=0500 ./bin/*.sh /opt/scripts/

# Add utilities
ENV ADDONS_HOME="/opt/openark/vdi/desktop"
ADD ./addons "${ADDONS_HOME}"
