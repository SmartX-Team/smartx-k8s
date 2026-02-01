{{- /*
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

# Kubernetes cluster version
kube_version: {{ .Values.build.kube.version | quote }}

{{- if and
  ( has "org.ulagbulag.io/cni" .Values.features )
  ( not .Values.cluster.services.kubeProxy.enabled )
}}
## List of kubeadm init phases that should be skipped during control plane setup
## By default 'addon/coredns' is skipped
## 'addon/kube-proxy' gets skipped for some network plugins
kubeadm_init_phases_skip_default:
  - addon/coredns
  - addon/kube-proxy
{{- end }}

# DNS configuration.
# Kubernetes cluster name, also will be used as DNS domain
cluster_name: {{ include "helm.clusterDomainName" $ | quote }}

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

## List of key=value pairs that describe feature gates for
## the k8s cluster.
{{- $_ := set $ "AvailableKubeadmFeatures" ( list
  "ControlPlaneKubeletLocalMode"
  "NodeLocalCRISocket"
  "WaitForAllControlPlaneComponents"
) }}

{{- $_ := set $ "KubeFeatures" ( list
  "ControlPlaneKubeletLocalMode"
  "ImageVolume"
  "PodLevelResources"
  "ProcMountType"
  "ResourceHealthStatus"
  "RotateKubeletServerCertificate"
  "UserNamespacesSupport"
) }}

{{- if has "org.ulagbulag.io/acceleration" .Values.features }}
{{- $_ := set $ "KubeFeatures" ( concat $.KubeFeatures ( list
  "CPUManagerPolicyAlphaOptions"
  "DRAResourceClaimDeviceStatus"
  "DynamicResourceAllocation"
  "HPAScaleToZero"
  "InPlacePodVerticalScaling"
  "InPlacePodVerticalScalingAllocatedStatus"
  "InPlacePodVerticalScalingExclusiveCPUs"
  "KubeletPodResourcesDynamicResources"
  "MemoryQoS"
  "SchedulerAsyncPreemption"
) ) }}
{{- end }}

kube_feature_gates: []
kube_proxy_feature_gates: []

{{- if $.KubeFeatures }}
{{- $_ := set $ "KubeFeatures" ( $.KubeFeatures | uniq | sortAlpha ) }}

kube_apiserver_feature_gates:
{{- range $_ := $.KubeFeatures }}
{{- if not ( has . $.AvailableKubeadmFeatures ) }}
  - {{ printf "%s=true" . | quote }}
{{- end }}
{{- end }}
kube_controller_feature_gates:
{{- range $_ := $.KubeFeatures }}
{{- if not ( has . $.AvailableKubeadmFeatures ) }}
  - {{ printf "%s=true" . | quote }}
{{- end }}
{{- end }}
kube_scheduler_feature_gates:
{{- range $_ := $.KubeFeatures }}
{{- if not ( has . $.AvailableKubeadmFeatures ) }}
  - {{ printf "%s=true" . | quote }}
{{- end }}
{{- end }}
kubelet_feature_gates:
{{- range $_ := $.KubeFeatures }}
{{- if not ( has . $.AvailableKubeadmFeatures ) }}
  - {{ printf "%s=true" . | quote }}
{{- end }}
{{- end }}
kubeadm_feature_gates:
{{- range $_ := $.KubeFeatures }}
{{- if has . $.AvailableKubeadmFeatures }}
  - {{ printf "%s=true" . | quote }}
{{- end }}
{{- end }}

#####################################
# k8s-cluster / kubelet / advanced
#####################################

{{- if has "org.ulagbulag.io/acceleration" .Values.features }}
## NUMA-aware scheduling
kubelet_cpu_manager_policy: static
kubelet_cpu_manager_policy_options:
  align-by-socket: "true"
  distribute-cpus-across-numa: "true"
  full-pcpus-only: "true"
  strict-cpu-reservation: "true"
{{- end }}

#####################################
# kubernetes / node
#####################################

{{- if has "org.ulagbulag.io/acceleration" .Values.features }}
# Set to empty to avoid cgroup creation
kubelet_enforce_node_allocatable: pods,kube-reserved,system-reserved

# Set systemd service hardening features
kubelet_systemd_hardening: true

# Reserve this space for kube resources
# Whether to run kubelet and container-engine daemons in a dedicated cgroup. (Not required for resource reservations).
kube_reserved: true

# Set to true to reserve resources for system daemons
system_reserved: true
{{- end }}

{{- end }}

{{- end }}
