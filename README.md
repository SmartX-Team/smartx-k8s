# SmartX K8S Repository

## System Requirements

Each node should have these **minimal requirements** below:

- CPU: 4 Cores
- Memory (RAM): 8Gi
- Local Storage: 40Gi

### Cgroup-wide Requirements

Please browse [apps/openark-kiss/values.yaml](apps/openark-kiss/values.yaml) for more information.

#### Control Plane + ETCD Nodes

| group        | CPU   | Memory | Ephemeral Storage | PIDs | Children                                   |
| ------------ | ----- | ------ | ----------------- | ---- | ------------------------------------------ |
| system.slice | 1000m | 5Gi    | 2Gi               | 4000 | etcd.service, systemd-systemd.service, ... |
| kube.slice   | 1000m | 2Gi    | 2Gi               | 2000 | sshd.service, ...                          |
| (Daemonsets) | 2000m | 1Gi    | 2Gi               | 2000 | CSI, GPU, KubeVirt, ...                    |

#### Worker Nodes

Please browse [apps/openark-kiss/values.yaml](apps/openark-kiss/values.yaml) for more information.

| group        | CPU   | Memory | Ephemeral Storage | PIDs | Children                     |
| ------------ | ----- | ------ | ----------------- | ---- | ---------------------------- |
| system.slice | 1000m | 2Gi    | 2Gi               | 4000 | systemd-systemd.service, ... |
| kube.slice   | 1000m | 2Gi    | 2Gi               | 2000 | sshd.service, ...            |
| (Daemonsets) | 2000m | 1Gi    | 2Gi               | 2000 | CSI, GPU, KubeVirt, ...      |

## OpenARK

### KISS: Kubernetes Is Short and Simple

Kubernetes makes it easy and efficient to manage resources in your clusters. Of course, this does not pertain to hardware resources that you airlift yourself!

For automated multi-site deployment of cloud-native on-premise (edge) clusters, you can run high-performance, real-time operations called `D.N.A` (aka. Data, Network, AI). However, in order for `D.N.A` to be managed automatically, easily and efficiently, you need to do extra work on Kubernetes directly. Are you capable of professional cluster fine-tuning?

[OpenARK KISS](/apps/openark-kiss) has appeared to solve these concerns. **Simply deploy and enjoy a self-managed edge clusters!** The services you create can be realized and accelerated anywhere, from embedded to KISS-based HPC, including mobile, depending on its resource requirements.

#### Note

`OpenARK KISS` is a part of `OpenARK`, which was designed to be used for research of [GIST's `NetAI` Laboratory](https://netai.smartx.kr/), led by [**Ho Kim**](https://github.com/kerryeon) under professor [**JongWon Kim**](https://netai.smartx.kr/people/professor). That is, this repository is operated for academic purposes, and there is **No commercial support in this repository**.

### VINE: VINE Is Not an Ecosystem

[OpenARK VINE Dashboard](/apps/openark-vine-dashboard) is a cloud-native dashboard GUI provisioning tool.

[OpenARK VINE Session](/apps/openark-vine-session-operator) is a cloud-native PC provisioning tool, which supports desktop-environment over Container and VM.

### References

- **Ho Kim**, DongHwan Ku and JongWon Kim, "Cloud-native Metadata Lake using OpenCAS, " in Proc. KICS (Korea Institute of Communications and Information Sciences) 2022 Winter Conference, Pyeongchang, Korea, February, 2022.
- **Ho Kim** and JongWon Kim, "Automated Multi-site Deployment of Bare-metal Cloud-native Edge Clusters, " in Proc. KICS (Korea Institute of Communications and Information Sciences) 2021 Fall Conference, Yeosu, Korea, November, 2021.
- **Ho Kim**, Jun-Sik Shin, and JongWon Kim, "Prototype Implementation of All-flash DataPond Cluster employing OpenCAS Cache Acceleration with Optane Memory, " in Proc. KICS (Korea Institute of Communications and Information Sciences) 2021 Summer Conference, Jeju, Korea, June, 2021.
