# Customizing Presets

1. Fork the preset repository (e.g., _`desktop-k8s`_ -> _`my-org/my-repo`_)
2. Edit files:
   - `manifest.yaml`
   - `values.yaml`
     1. `auth`: Authorization and Authentication
     2. `bootstrapper`: Bootstrapper nodes (e.g. SSH, clean install, ...)
     3. `cluster`: Kubernetes cluster configuration (e.g. cluster name, group, cluster IPs, region, ...)
     4. `driver`: H/W-specific driver configuration (e.g. NVIDIA GPU, ...)
     5. `features`: Optional features to enable. A feature may install required applications, enabling features of apps, etc.
     6. `ingress`: Gateway and Ingress configuration (e.g. base domain name, nameservers, ...)
     7. `kiss`: Bare-metal kubernetes cluster configuration (e.g. ETCD, node OS and version, IPMI, ...)
     8. `kubespray`: Optional inventory for `kubespray` playbook, applied to all
     9. `network`: Bare-metal network configuration (e.g. MTU, DHCP, Wi-Fi, NMS (SNMP, LLDP), ...)
     10. `optimization`: Cluster performance configuration, based on `tuned`, `cgroups` and k8s `cpuManager`
     11. `tower`: Multi-cluster configuration (e.g. observability, cluster mesh, ...)
     12. `twin`: Digital-twin configuration (e.g. Robotics, AI-powered orchestration, ...)
     13. `vine`: Desktop environment configuration (e.g. `nodeSelector`, ...)
   - `apps/*/values.yaml`
3. Use your custom URL with the `just` command:

```bash
# If you want to bootstrap a k8s cluster via SSH,
just bootstrap "https://github.com/your-org/your-custom-k8s"

# If you want to build an ISO image,
just build-iso "https://github.com/your-org/your-custom-k8s"
```
