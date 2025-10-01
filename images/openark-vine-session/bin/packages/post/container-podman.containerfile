# Set default environment variables
ENV _CONTAINERS_USERNS_CONFIGURED=""
ENV PATH="${PATH}:/usr/games:/usr/local/bin:/opt/bin"

# Add config files
ADD --chmod=0644 ./etc/containers/nodocker /etc/containers/nodocker
ADD --chmod=0644 ./etc/containers/*.conf /etc/containers/
