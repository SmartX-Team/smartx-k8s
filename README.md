# SmartX K8S Repository

## System Requirements

Each node should have these **minimal requirements** below:

- CPU: 4 Cores
- Memory (RAM): 8Gi
- Local Storage: 40Gi

### Cgroup-wide Requirements

Please browse [apps/openark-kiss/values.yaml](apps/openark-kiss/values.yaml) for more information.

| group        | CPU   | Memory | Ephemeral Storage | PIDs | Children                     |
| ------------ | ----- | ------ | ----------------- | ---- | ---------------------------- |
| system.slice | 1000m | 5Gi    | 2Gi               | 4000 | systemd-systemd.service, ... |
| kube.slice   | 1000m | 2Gi    | 2Gi               | 2000 | sshd.service, ...            |
