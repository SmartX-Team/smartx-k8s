
{{/*
Derived Kubespray Configs
*/}}
{{- define "helm.kubesprayConfigs" -}}

#####################################
# kubernetes / control-plane
#####################################

## Enable/Disable Kube API Server Authentication Methods
kube_oidc_auth: {{ has "org.ulagbulag.io/auth" .Values.features }}

{{- if has "org.ulagbulag.io/auth" .Values.features }}
## Variables for OpenID Connect Configuration https://kubernetes.io/docs/admin/authentication/
## To use OpenID you have to deploy additional an OpenID Provider (e.g Dex, Keycloak, ...)
kube_oidc_url: "https://{{ .Values.auth.domainName | default ( printf "auth.%s" .Values.ingress.domainName ) }}/realms/{{ .Values.auth.realms.name }}"
{{- end }}

#####################################
# k8s-cluster / kubelet
#####################################

# DNS configuration.
# Kubernetes cluster name, also will be used as DNS domain
cluster_name: {{ .Values.cluster.domainName | quote }}

# Vars for pointing to kubernetes api endpoints
kube_apiserver_global_endpoint: {{ .Values.cluster.apiEndpoint | quote }}

# internal network. When used, it will assign IP
# addresses from this range to individual pods.
# This network must be unused in your network infrastructure!
kube_pods_subnet: {{ .Values.cluster.pods.ipv4.subnet | quote }}
kube_child_pods_subnet: {{ .Values.cluster.pods.ipv4.childSubnet | quote }}

# internal network node size allocation (optional). This is the size allocated
# to each node for pod IP address allocation. Note that the number of pods per node is
# also limited by the kubelet_max_pods variable which defaults to 110.
#
# Example:
# Up to 64 nodes and up to 254 or kubelet_max_pods (the lowest of the two) pods per node:
#  - kube_pods_subnet: 10.233.64.0/18
#  - kube_network_node_prefix: 24
#  - kubelet_max_pods: 110
#
# Example:
# Up to 128 nodes and up to 126 or kubelet_max_pods (the lowest of the two) pods per node:
#  - kube_pods_subnet: 10.233.64.0/18
#  - kube_network_node_prefix: 25
#  - kubelet_max_pods: 110
kube_network_node_prefix: {{ .Values.cluster.pods.ipv4.prefix }}

# Internal network. When used, it will assign IPv6 addresses from this range to individual pods.
# This network must not already be in your network infrastructure!
# This is only used if enable_dual_stack_networks is set to true.
# This provides room for 256 nodes with 254 pods per node.
kube_pods_subnet_ipv6: {{ .Values.cluster.pods.ipv6.subnet | quote }}

# IPv6 subnet size allocated to each for pods.
# This is only used if enable_dual_stack_networks is set to true
# This provides room for 254 pods per node.
kube_network_node_prefix_ipv6: {{ .Values.cluster.pods.ipv6.prefix }}

# Kubernetes internal network for services, unused block of space.
kube_service_addresses: {{ .Values.cluster.services.ipv4.subnet | quote }}
kube_child_service_addresses: {{ .Values.cluster.services.ipv4.childSubnet | quote }}

kube_proxy_deployed: {{ .Values.cluster.services.kubeProxy.enabled }}
kube_proxy_remove: {{ not .Values.cluster.services.kubeProxy.enabled }}

# Metrics Server deployment
metrics_server_enabled: {{ has "org.ulagbulag.io/monitoring" .Values.features }}

## Upstream dns servers
upstream_dns_servers:
  - {{ .Values.network.nameservers.ns1 | quote }}
  - {{ .Values.network.nameservers.ns2 | quote }}

{{- end }}
