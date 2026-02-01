# SmartX K8S Repository

![Cluster Health (MobileX)](https://cd.argo.mobilex.kr/api/badge?name=mobilex&revision=true&showAppName=true&namespace=argo)

This repository provides an integrated framework for deploying and managing [Kubernetes](https://kubernetes.io) clusters with the following key capabilities.
It enables [automated cluster provisioning](/docs/openark.md#kiss-kubernetes-is-short-and-simple), supports [desktop-class Kubernetes environments](/docs/openark.md#vine-vine-is-not-an-ecosystem), and [bridges desktop and HPC infrastructures](https://github.com/SmartX-Team/Connected-Data-Lake).
[A decentralized data lake and storage system](https://github.com/SmartX-Team/Connected-Data-Lake) is included, along with [a streamlined GitOps setup](/apps/argo-workflows/) using [Argo CD](https://argoproj.github.io/cd/) and [Helm](https://helm.sh/) through the [App of Apps pattern](https://argo-cd.readthedocs.io/en/stable/operator-manual/cluster-bootstrapping/).
The repository also introduces [an abstract application model—similar in spirit to Java interfaces](/docs/presets.md)—allowing users to define and inherit [custom preset repositories](https://github.com/SmartX-Team/desktop-k8s) for modular application composition.
Robust [CI](/apps/argo-workflows/)/[CD](/apps/argo-cd/) integration and a suite of developer-friendly features further strengthen its usability in production environments, which is why it was considered effective for scalable infrastructure management.

## Documents

- [Building container images on local](/docs/build-images.md)
- [Continuous Integration (CI) of this repository](/docs/ci.md)
- [Customizing Presets](/docs/presets.md)
- [Framework Structure and Key Components](/docs/framework.md)
- [How to control the bare-metal nodes manually?](/docs/bare-metal-node.md)
- [System Dependencies](/docs/dependencies.md)
- [System Requirements](/docs/requirements.md)
- [What is `OpenARK`?](/docs/openark.md)

## Getting Started

The build instructions below are designed for developers and advanced users.
If you want to use them for general purposes, please download and use the ISO images in `releases`.

[You can check the proper system requirements here.](/docs/requirements.md)

[We suggest to install the dependencies in your client node (e.g. Laptops).](/docs/dependencies.md)

### Bootstrap mode (Remote via SSH)

This method installs SmartX K8S onto remote nodes using SSH.

```bash
just bootstrap "https://github.com/SmartX-Team/desktop-k8s"
```

- Once the task is complete, you will have a **Kubernetes cluster** installed.
- The installation process is fully automatic, depending on the configuration of the preset. If the desktop feature `org.ulagbulag.io/desktop-environment/vine` is activated, the OS installation and desktop environment setup are performed automatically. This process takes about an hour, depending on the Internet connection performance and local node performance.
- **Custom Cluster Configuration**

  ```bash
    just bootstrap "https://github.com/your-org/your-custom-k8s"
  ```

### Standalone mode (Build an ISO)

This method converts your desktop or server into a local control-plane node.

```bash
just build-iso "https://github.com/SmartX-Team/desktop-k8s"
```

- This generates a **bootable ISO image** for [installing SmartX K8S onto bare-metal](/apps/openark-kiss/).
- The installation process is fully automatic, depending on the configuration of the preset. If the desktop feature `org.ulagbulag.io/desktop-environment/vine` is activated, the OS installation and desktop environment setup are performed automatically. This process takes about an hour, depending on the Internet connection performance and local node performance.
- **Custom ISO Configuration**

  ```bash
  just build-iso "https://github.com/your-org/your-custom-k8s"
  ```

## Features

- [Automated Kubernetes cluster provisioning](/apps/openark-kiss/)
- [Support for desktop-class cluster environments (Desktop As A Service)](/apps/openark-vine-session-operator/)
- Integration of desktop and HPC infrastructures into a kubernetes cluster
- [Decentralized data lake and storage system](/apps/data-pond/)
- Simplified GitOps deployment using Argo CD + Helm with the App of Apps pattern
- [Abstract application model (akin to Java interfaces) for creating reusable and extensible preset repositories](/docs/presets.md)
- Instant and Seamless patches of `Kernel & K8S & Apps` (e.g. CVE, ...)
- Vendor-neutric [CI](/apps/argo-workflows/)/[CD](/apps/argo-cd/) integration and [developer-friendly features](/hack/) for a robust GitOps workflow
- [Lightweight and Customize linux kernel and OS](/images/openark-linux-kernel/)
- User-friendly Web UI/UX: [Customize desktops](https://github.com/ulagbulag/openark-desktop-template), [clusters](/docs/presets.md) and digital twin online
- Fully Open-Source Software under [GPL-3.0](/LICENSE)

# LICENSE

Please refer the [LICENSE](/LICENSE) file.
