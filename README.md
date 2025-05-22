# SmartX K8S Repository

This repository provides an integrated framework for deploying and managing Kubernetes clusters with the following key capabilities.
It enables automated cluster provisioning, supports desktop-class Kubernetes environments, and bridges desktop and HPC infrastructures.
A decentralized data lake and storage system is included, along with a streamlined GitOps setup using Argo CD and Helm through the App of Apps pattern.
The repository also introduces an abstract application model—similar in spirit to Java interfaces—allowing users to define
and inherit custom preset repositories for modular application composition.
Robust CI/CD integration and a suite of developer-friendly features further strengthen its usability in production environments,
which is why it was considered effective for scalable infrastructure management.

- Automated Kubernetes cluster provisioning
- Support for desktop-class cluster environments (Desktop As A Service)
- Integration of desktop and HPC infrastructures into a kubernetes cluster
- Decentralized data lake and storage system
- Simplified GitOps deployment using Argo CD + Helm with the App of Apps pattern
- Abstract application model (akin to Java interfaces) for creating reusable and extensible preset repositories
- Instant and Seamless patches of `Kernel & K8S & Apps` (e.g. CVE, ...)
- Vendor-neutric CI/CD integration and developer-friendly features for a robust GitOps workflow
- Lightweight and Customize linux kernel and OS
- User-friendly Web UI/UX: Customize desktops, clusters and digital twin online
- Fully Open-Source Software under [GPL-3.0](/LICENSE)

## Getting Started

The build instructions below are designed for developers and advanced users.
If you want to use them for general purposes, please download and use the ISO images in `releases`.

### System Requirements

Each node should have these **minimal requirements** below:

- CPU: 4 vCores
- Memory (RAM): 8Gi
- Local Storage: 40Gi

We suggest to fit the **recommended requirements** below:

- CPU: 8 vCores
- Memory (RAM): 16Gi
- Local Storage: 200Gi

We suggest to install the dependencies in your client node (e.g. Laptops) like below:

#### Debian/Ubuntu

```bash
sudo apt-get update && sudo apt-get install -y \
    7zip \
    bash \
    curl \
    git \
    ipcalc \
    just \
    podman \
    yq
```

#### What is `just`?

`just` is a command runner similar to make, used to simplify repetitive CLI tasks via predefined commands stored in a Justfile.

You can find more information and instrutions at "https://github.com/casey/just".

### Bootstrap mode (Remote via SSH)

This method installs SmartX K8S onto remote nodes using SSH.

```bash
just bootstrap "https://github.com/SmartX-Team/desktop-k8s"
```

#### Custom Preset

You may provide a custom GitHub repository URL if you want to customize the cluster setup:

```bash
just bootstrap "https://github.com/your-org/your-custom-k8s"
```

### Standalone mode (Build an ISO)

This method converts your desktop or server into a local control-plane node.

```bash
just build-iso "https://github.com/SmartX-Team/desktop-k8s"
```

- This generates a **bootable ISO image** for installing SmartX K8S onto bare-metal.
- The installation process is fully automatic, depending on the configuration of the preset. If the desktop feature `org.ulagbulag.io/desktop-environment/vine` is activated, the OS installation and desktop environment setup are performed automatically. This process takes about an hour, depending on the Internet connection performance and local node performance.
- **Custom ISO Configuration**

```bash
just build-iso "https://github.com/your-org/your-custom-k8s"
```

## Framework Structure and Key Components

### App of Apps Pattern

- Manages multiple Argo CD applications using Helm
- Uses a layered GitOps structure with centralized control
- Supports modular configuration via reusable preset repositories

### [OpenARK](#appendix-b-openark) Components

- **KISS**: A simplified Kubernetes configuration for edge clusters
- **Spectrum API**: A `Histogram API` to classify the services by metrics and a `Pool API` to provisioning the services without handling containers, compatible with `Gateway API`
- **VINE Dashboard**: A web-based UI for managing resources and sessions
- **VINE Session Operator**: Desktop-as-a-Service implementation over containers and VMs

## Customizing Presets

1. Fork the preset repository (e.g., desktop-k8s)
2. Edit files:
   - `manifest.yaml`
   - `values.yaml`
     1. `auth`: Authorization and Authentication
     2. `bootstrapper`: Bootstrapper nodes (e.g. SSH, clean install, ...)
     3. `cluster`: Kubernetes cluster configuration (e.g. cluster name, group, cluster IPs, region, ...)
     4. `driver`: H/W-specific driver configuration (e.g. NVIDIA GPU, ...)
     5. `features`: Optional features to enable. A feature may install required applications, enabling features of apps, etc.
     6. `ingress`: Gateway and Ingress configuration (e.g. base domain name, nameservers, ...)
     7. `kiss`: Bare-metal kubernetes cluster configuration (e.g. ETCD, `kubespray` version, node OS and version, IPMI, ...)
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

# Appendix A. Cgroup-wide Requirements

Please browse [apps/openark-kiss/values.yaml](apps/openark-kiss/values.yaml) for more information.

## Control Plane + ETCD Nodes

| group        | CPU   | Memory | Ephemeral Storage | PIDs | Children                                   |
| ------------ | ----- | ------ | ----------------- | ---- | ------------------------------------------ |
| system.slice | 1000m | 5Gi    | 2Gi               | 4000 | etcd.service, systemd-systemd.service, ... |
| kube.slice   | 1000m | 2Gi    | 2Gi               | 2000 | sshd.service, ...                          |
| (Daemonsets) | 2000m | 1Gi    | 2Gi               | 2000 | CSI, GPU, KubeVirt, ...                    |

## Worker Nodes

Please browse [apps/openark-kiss/values.yaml](apps/openark-kiss/values.yaml) for more information.

| group        | CPU   | Memory | Ephemeral Storage | PIDs | Children                     |
| ------------ | ----- | ------ | ----------------- | ---- | ---------------------------- |
| system.slice | 1000m | 2Gi    | 2Gi               | 4000 | systemd-systemd.service, ... |
| kube.slice   | 1000m | 2Gi    | 2Gi               | 2000 | sshd.service, ...            |
| (Daemonsets) | 2000m | 1Gi    | 2Gi               | 2000 | CSI, GPU, KubeVirt, ...      |

# Appendix B. OpenARK

## KISS: Kubernetes Is Short and Simple

Kubernetes makes it easy and efficient to manage resources in your clusters. Of course, this does not pertain to hardware resources that you airlift yourself!

For automated multi-site deployment of cloud-native on-premise (edge) clusters, you can run high-performance, real-time operations called `D.N.A` (aka. Data, Network, AI). However, in order for `D.N.A` to be managed automatically, easily and efficiently, you need to do extra work on Kubernetes directly. Are you capable of professional cluster fine-tuning?

[OpenARK KISS](/apps/openark-kiss) has appeared to solve these concerns. **Simply deploy and enjoy a self-managed edge clusters!** The services you create can be realized and accelerated anywhere, from embedded to KISS-based HPC, including mobile, depending on its resource requirements.

### Note

`OpenARK KISS` is a part of `OpenARK`, which was designed to be used for research of [GIST's `NetAI` Laboratory](https://netai.smartx.kr/), led by [**Ho Kim**](https://github.com/kerryeon) under professor [**JongWon Kim**](https://netai.smartx.kr/people/professor).
That is, this repository is operated for academic purposes, and there is **No commercial support in this repository AS IS**.
[If you would like commercial support, please contact us through the lab website.](https://netai.smartx.kr/)

## VINE: VINE Is Not an Ecosystem

[OpenARK VINE Dashboard](/apps/openark-vine-dashboard) is a cloud-native dashboard GUI provisioning tool.

[OpenARK VINE Session](/apps/openark-vine-session-operator) is a cloud-native PC provisioning tool, which supports desktop-environment over Container and VM.

## References

- **Ho Kim**, DongHwan Ku and JongWon Kim, "Cloud-native Metadata Lake using OpenCAS, " in Proc. KICS (Korea Institute of Communications and Information Sciences) 2022 Winter Conference, Pyeongchang, Korea, February, 2022.
- **Ho Kim** and JongWon Kim, "Automated Multi-site Deployment of Bare-metal Cloud-native Edge Clusters, " in Proc. KICS (Korea Institute of Communications and Information Sciences) 2021 Fall Conference, Yeosu, Korea, November, 2021.
- **Ho Kim**, Jun-Sik Shin, and JongWon Kim, "Prototype Implementation of All-flash DataPond Cluster employing OpenCAS Cache Acceleration with Optane Memory, " in Proc. KICS (Korea Institute of Communications and Information Sciences) 2021 Summer Conference, Jeju, Korea, June, 2021.

# LICENSE

Please refer the [LICENSE](/LICENSE) file.
