# Requirements

We cannot guarantee smooth operation in all machine environments.
However, we strive to ensure proper operation on machines that meet the following conditions.
Even if these conditions do not apply, we are expanding the environment through [Issues](https://github.com/SmartX-Team/smartx-k8s/issues) or machine donations.

## System Requirements

Each node should have these **minimal requirements** below:

- CPU: 4 vCores
- Memory (RAM): 8Gi
- Local Storage: 40Gi

We suggest to fit the **recommended requirements** below:

- CPU: 8 vCores
- Memory (RAM): 16Gi
- Local Storage: 500Gi

We have tested on these nodes below:

| Vendor                       | Product            | Arch    | CPU                                                     | RAM   | Primary GPU                        |
| ---------------------------- | ------------------ | ------- | ------------------------------------------------------- | ----- | ---------------------------------- |
| ASUSTeK COMPUTER INC.        | NUC15JNKU9X9       | x86_64  | Intel(R) Core(TM) Ultra 9 275HX                         | 62Gi  | NVIDIA GeForce RTX 5080 Laptop GPU |
| Intel Corporation            | S2600WT2           | x86_64  | Intel(R) Xeon(R) CPU E5-2640 v3 @ 2.60GHz               | 62Gi  | -                                  |
| Intel(R) Client Systems      | NUC10i7FNH         | x86_64  | Intel(R) Core(TM) i7-10710U CPU @ 1.10GHz               | 15Gi  | Intel® UHD Graphics                |
| Intel(R) Client Systems      | NUC11BTMi9         | x86_64  | 11th Gen Intel(R) Core(TM) i9-11900KB @ 3.30GHz         | 62Gi  | NVIDIA GeForce RTX 3070            |
| Intel(R) Client Systems      | NUC12DCMv7         | x86_64  | 12th Gen Intel(R) Core(TM) i7-12700                     | 62Gi  | NVIDIA GeForce RTX 3070            |
| Intel(R) Client Systems      | NUC13RNGi7         | x86_64  | 13th Gen Intel(R) Core(TM) i7-13700K                    | 62Gi  | NVIDIA GeForce RTX 4070            |
| NVIDIA                       | DGX-A100           | x86_64  | AMD EPYC 7742 64-Core Processor                         | 1.0Ti | NVIDIA A100 SXM4 40GB \* 8         |
| NVIDIA                       | DGX-Spark          | aarch64 | 20-core Arm processor (10 Cortex-X925 + 10 Cortex-A725) | 119Gi | NVIDIA GB10                        |
| Quanta Cloud Technology Inc. | QuantaGrid D52G-4U | x86_64  | Intel(R) Xeon(R) Gold 5118 CPU @ 2.30GHz                | 125Gi | NVIDIA A10 \* 10                   |
| Supermicro                   | Super Server       | x86_64  | Intel(R) Xeon(R) Silver 4215R CPU @ 3.20GHz             | 31Gi  | -                                  |
| Supermicro                   | SYS-1029U-TN10RT   | x86_64  | Intel(R) Xeon(R) Bronze 3204 CPU @ 1.90GHz              | 62Gi  | -                                  |
| Supermicro                   | SYS-2029BZ-HNR     | x86_64  | Intel(R) Xeon(R) Silver 4215R CPU @ 3.20GHz             | 31Gi  | -                                  |

## Cgroup-wide Requirements

Please browse [apps/openark-kiss/values.yaml](/apps/openark-kiss/values.yaml) for more information.

### Control Plane + ETCD Nodes

| group        | CPU   | Memory | Ephemeral Storage | PIDs | Children                                   |
| ------------ | ----- | ------ | ----------------- | ---- | ------------------------------------------ |
| system.slice | 1000m | 5Gi    | 2Gi               | 4000 | etcd.service, systemd-systemd.service, ... |
| kube.slice   | 1000m | 2Gi    | 2Gi               | 2000 | sshd.service, ...                          |
| (Daemonsets) | 2000m | 1Gi    | 2Gi               | 2000 | CSI, GPU, KubeVirt, ...                    |

### Worker Nodes

Please browse [apps/openark-kiss/values.yaml](/apps/openark-kiss/values.yaml) for more information.

| group        | CPU   | Memory | Ephemeral Storage | PIDs | Children                     |
| ------------ | ----- | ------ | ----------------- | ---- | ---------------------------- |
| system.slice | 1000m | 2Gi    | 2Gi               | 4000 | systemd-systemd.service, ... |
| kube.slice   | 1000m | 2Gi    | 2Gi               | 2000 | sshd.service, ...            |
| (Daemonsets) | 2000m | 1Gi    | 2Gi               | 2000 | CSI, GPU, KubeVirt, ...      |
