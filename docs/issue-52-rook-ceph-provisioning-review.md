# SmartX-K8S Issue #52 독립 아키텍처·코드 리뷰

검토 대상은 Issue [#52](https://github.com/SmartX-Team/smartx-k8s/issues/52), 기능 브랜치 커밋
`4be531d3d8b45bf959d1841c4408c62050818e5f`, 그리고 그 위의 로컬 dry-run 기본값 수정이다.
검토일은 2026-08-27이다.

## A. Executive verdict

**판정: `KEEP`. 단, 기본 비활성이고 기본 dry-run인 실험적 plumbing으로만 유지한다.**

기능 브랜치의 핵심 방향은 Issue #52에 맞는다.

- 기존 Rook 추상화를 대체하지 않고 `CephCluster.spec.storage.nodes[].devices[]`를 생성한다.
- KISS `Storage` 노드에서만 로컬 장치 증거를 수집한다.
- 하드웨어 사실과 자식 클러스터의 Ceph 정책 매핑을 분리한다.
- 기존 OSD 항목은 보존하며 새 장치만 추가 대상으로 삼는다.
- 기능과 런타임 소유권은 명시적으로 선택해야 한다.
- 차트는 기본 비활성이고 Helm의 `controller.dryRun`도 기본 `true`다
  ([values.yaml](../apps/rook-ceph-provisioning/values.yaml#L1-L9),
  [values.yaml](../apps/rook-ceph-provisioning/values.yaml#L40-L46)).

로컬 수정은 controller 바이너리를 차트 밖에서 직접 실행해도 같은 안전 기본값을 갖게 한다.
`Args.dry_run`의 Clap 기본값을 `false`에서 `true`로 바꾸고,
환경 변수와 CLI override가 없을 때 `true`인지 확인하는 parser 회귀 테스트를 추가했다
([openark-rook-ceph-controller.rs](../crates/openark-rook-ceph-controller/src/bin/openark-rook-ceph-controller.rs#L73-L77),
  [openark-rook-ceph-controller.rs](../crates/openark-rook-ceph-controller/src/bin/openark-rook-ceph-controller.rs#L1157-L1182)).

이 수정과 아래 검증을 반영한 현재 브랜치는 로컬 push 및 PR 리뷰 대상으로 적합하다.
그 의미는 설계와 비파괴 dry-run 경로를 리뷰할 수 있다는 뜻이지,
운영 자동 등록을 승인한다는 뜻이 아니다.

**운영 `dryRun=false`는 지원하지 않는다.**
agent가 열린 file descriptor에 묶어 서명을 검사한 장치 신원은 ConfigMap 게시 전에 닫히며,
그 descriptor나 물리 장치 claim은 controller 또는 Rook prepare 작업으로 전달되지 않는다.
따라서 관측 후 Rook 실행 전에 `/dev/disk/by-id` alias가 빈 교체 장치로 재지정되면
현재 검사를 통과한 뒤 다른 물리 장치가 준비될 수 있다.
이 actuation-time TOCTOU가 해결되고 실제 장치 및 API server E2E가 추가되기 전까지
운영 쓰기는 차단해야 한다.

## B. Requirements and invariants

### B.1 Issue가 요구하는 결과

Issue #52의 요구사항과 본 브랜치가 선택한 구현 방식은 구분해야 한다.
Issue 본문은 두 가지 integration point를 비교 대상으로 제시했으며,
cluster-local controller 자체를 고정 요구하지 않았다
([Issue #52](https://github.com/SmartX-Team/smartx-k8s/issues/52)).

Issue에서 직접 도출되는 요구사항은 다음과 같다.

1. **Opt-in 동작**
   기능을 끄면 기존의 단순한 Rook-Ceph 동작이 바뀌지 않아야 한다.
   현재 루트 값은 `enabled: false`, `manageStorageNodes: false`다
   ([values.yaml](../values.yaml#L397-L408)).
2. **Kubernetes cluster당 CephCluster 하나**
   tower/default cluster와 `datax` 같은 child Kubernetes cluster가 각자의 CephCluster를 가질 수 있다.
   한 Kubernetes cluster 안의 여러 CephCluster 지원은 이 Issue 범위가 아니다.
3. **재사용 가능한 discovery와 classification**
   `smartx-k8s`는 장치 발견, media 분류, PCIe link 정규화,
   stable path 선택, 명시적 storage 목록 생성을 제공한다.
4. **Child-cluster policy mapping**
   `<cluster>-k8s`는 `deviceClassMap`, pool, replication, failure domain,
   MON/MGR placement와 성능 정책을 정의한다.
   generic discovery가 `capacity-tier` 같은 사이트 의미를 추론하면 안 된다.
5. **명시적 nodes/devices**
   `useAllNodes: false`, `useAllDevices: false` 아래에서
   `storage.nodes[].devices[]`를 생성해야 한다.
   Rook 문서도 이 모드에서 node 이름이 `kubernetes.io/hostname`과 맞아야 하고,
   explicit udev link를 장치 이름으로 지원한다고 설명한다
   ([Rook explicit devices](https://rook.io/docs/rook/v1.19/CRDs/Cluster/host-cluster/#specific-nodes-and-devices)).
6. **새 장치만 자동화**
   기존 OSD의 class 변경이나 CRUSH 재분류는 초기 범위가 아니다.
   기존 항목의 삭제, 이동, class 변경도 자동화하지 않는다.
7. **Stable path**
   생성 항목은 `/dev/nvme0n1` 같은 커널 순번보다 결정적으로 선택한
   `/dev/disk/by-id/...`를 사용한다.
   이 alias는 편리한 지속 경로이지 암호학적 신원이나 장치 예약은 아니다.
8. **Argo ownership**
   runtime이 `spec.storage.nodes`를 소유하면 Argo self-heal이 그 값을 되돌리지 않아야 한다.
   ignore rule과 `RespectIgnoreDifferences=true`를 함께 써야 sync 단계에도 반영된다
   ([Argo CD sync options](https://argo-cd.readthedocs.io/en/stable/user-guide/sync-options/),
   [manifest.yaml](../apps/rook-ceph-cluster/manifest.yaml#L32-L44)).

### B.2 브랜치가 제안하고 선택한 설계

다음 항목은 Issue의 고정 요구가 아니라 브랜치의 설계 결정이다.

- privileged node agent DaemonSet이 `/host/dev`와 `/host/sys`를 read-only로 보고 inventory를 게시한다
  ([daemonset-agent.yaml](../apps/rook-ceph-provisioning/templates/daemonset-agent.yaml#L36-L49),
  [daemonset-agent.yaml](../apps/rook-ceph-provisioning/templates/daemonset-agent.yaml#L78-L100)).
- 단일 non-root controller Deployment가 Rook discovery와 agent inventory를 교차 확인한다
  ([deployment-controller.yaml](../apps/rook-ceph-provisioning/templates/deployment-controller.yaml#L12-L17),
  [deployment-controller.yaml](../apps/rook-ceph-provisioning/templates/deployment-controller.yaml#L48-L57)).
- controller는 기존 storage 객체를 읽고 explicit node/device 목록 전체를 계산한다.
- `resourceVersion`을 넣은 JSON Merge Patch로 `spec.storage.nodes` 배열 전체를 조건부 교체한다.
- 모든 eligible node와 기존 Ceph device의 현재 증거가 완전할 때만 계획을 만든다.
- 어느 노드에서든 증거가 빠지거나 충돌하면 cluster 전체 reconciliation을 중단한다.

### B.3 안전 불변식

이 실험적 plumbing을 유지하려면 아래 불변식을 약화하면 안 된다.

| 불변식 | 현재 보장 | 남은 한계 |
|---|---|---|
| 명시적 opt-in | 두 flag와 chart enable 조건 | 운영 쓰기 승인은 별도임 |
| 대상 노드 제한 | KISS Storage label/value 일치 | label 정책 자체는 cluster 운영 책임 |
| 선택 범위 | `useAllNodes=false`, `useAllDevices=false`, broad selector 거부 | runtime이 전체 list를 독점 소유해야 함 |
| 신규 장치 한정 | Rook와 agent 양쪽의 available/signature-free 증거 요구 | 관측과 Rook prepare 사이 TOCTOU |
| 기존 OSD 보존 | 기존 node/device 객체와 추가 필드를 그대로 출력에 복사 | 기존 상태가 불완전하면 전체 reconcile 중단 |
| 교차 노드 중복 방지 | 모든 by-id claim을 cluster 단위로 비교 | durable claim이 없어 availability 희생 |
| 증거 신선도 | Pod UID, owner, timestamp, resourceVersion fingerprint 확인 | 게시 후 물리 descriptor를 보존하지 않음 |
| 동시 수정 방지 | CephCluster `resourceVersion` precondition | 장치 자체의 actuation claim은 아님 |
| GitOps 분리 | `/spec/storage/nodes` 전체 ignore | list entry별 SSA 소유권 분리는 불가능 |
| 안전 기본값 | chart와 direct binary 모두 dry-run `true` | 명시적 `false`는 기술적으로 가능하나 지원하지 않음 |

## C. Findings table

심각도는 데이터 손상 가능성, 운영 차단 여부, 그리고 Issue 범위를 기준으로 정렬했다.

| 심각도 | 상태 | Finding | 근거와 처분 |
|---|---|---|---|
| **HIGH** | **FIXED** | direct binary가 live mutation을 기본값으로 사용했다 | 차트는 `DRY_RUN=true`를 주입했지만 Clap 기본값은 `false`여서 바이너리 직접 실행 시 쓰기가 기본이었다. 로컬 수정은 `default_value_t = true`로 바꾸고 `dry_run_defaults_true_without_env_or_cli_override` parser 테스트를 추가했다. 이 수정은 merge 전 필수다. |
| **HIGH** | **RESIDUAL, WRITES BLOCKED** | fd-bound scan identity가 Rook preparation으로 전달되지 않는다 | agent는 alias를 열고 descriptor의 `rdev`와 sysfs를 확인한 상태에서 libblkid를 실행한다 ([device.rs](../crates/openark-rook-ceph-controller/src/device.rs#L198-L245)). libblkid가 전달받은 fd에 probe를 묶는 것은 upstream 구현과도 맞는다 ([libblkid fd probe](https://github.com/util-linux/util-linux/blob/f9347e0f0b5e4eace961470199fb00cd40ccb51c/libblkid/src/probe.c#L400-L490)). Linux fd 기반 stat은 열린 file을 기준으로 하지만 ([Linux fd stat semantics](https://github.com/torvalds/linux/blob/73e3f0710014fe6d4ed98cfc02292f6121db7558/fs/stat.c#L70-L120)), 그 fd는 snapshot 뒤 닫히고 게시되거나 Rook에 전달되지 않는다. Rook는 나중에 장치 이름과 현재 dev links를 다시 맞추고 ([Rook device matching](https://github.com/rook/rook/blob/349da958b24b6dddef275cd47e9f690d03dab6c3/pkg/daemon/ceph/osd/daemon.go#L504-L536)), 별도 raw prepare를 실행한다 ([Rook raw prepare](https://github.com/rook/rook/blob/349da958b24b6dddef275cd47e9f690d03dab6c3/pkg/daemon/ceph/osd/volume.go#L531-L574)). alias를 같은 크기와 속성의 빈 교체 장치로 재지정하면 현재 경계 사이를 통과할 수 있다. `dryRun=false`는 차단한다. |
| **MEDIUM** | **ACCEPTED** | `storage.nodes`와 nested `devices`는 atomic CRD list다 | Rook v1.19.8 schema의 두 list는 atomic topology로 정의된다 ([Rook v1.19.8 schema](https://github.com/rook/rook/blob/349da958b24b6dddef275cd47e9f690d03dab6c3/deploy/charts/rook-ceph/templates/resources.yaml#L3992-L4026)). Kubernetes SSA는 CRD schema의 list topology에 따라 ownership을 추적하므로 atomic list를 node/device entry별 manager로 나눌 수 없다 ([Kubernetes SSA custom resources](https://kubernetes.io/docs/reference/using-api/server-side-apply/#custom-resources-and-server-side-apply)). 따라서 전체 `/spec/storage/nodes` ignore와 runtime의 배타적 전체 필드 소유가 필요하다. per-entry SSA 전환을 권하지 않는다. |
| **MEDIUM** | **ACCEPTED** | cluster-wide fail-closed가 availability를 낮춘다 | 현재 controller는 eligible node set, agent Pod/ConfigMap set, Rook discovery, 기존 Ceph device coverage, persistent alias 중복을 모두 완전하게 요구한다 ([openark-rook-ceph-controller.rs](../crates/openark-rook-ceph-controller/src/bin/openark-rook-ceph-controller.rs#L741-L790), [openark-rook-ceph-controller.rs](../crates/openark-rook-ceph-controller/src/bin/openark-rook-ceph-controller.rs#L897-L968)). 한 노드 증거 실패가 다른 노드의 안전한 후보도 지연시킨다. durable claim 없이 부분 진행하면 cross-node duplicate 검사를 약화하므로 현재 trade-off를 유지한다. |
| **MEDIUM** | **PREREQUISITE** | Rook v1.19.6이 raw mode의 per-device `deviceClass` 전파를 처음 수정했다 | PR [#17407](https://github.com/rook/rook/pull/17407)은 prepare와 reconcile에서 per-device class가 무시되던 문제를 수정했고, [v1.19.6 release](https://github.com/rook/rook/releases/tag/v1.19.6)에 포함됐다. 브랜치가 고정한 v1.19.8은 이 수정을 포함하므로 유지해야 한다 ([manifest.yaml](../apps/rook-ceph-cluster/manifest.yaml#L45-L48), [v1.19.8 release](https://github.com/rook/rook/releases/tag/v1.19.8)). 이 bump는 정리용 변경이 아니라 기능 전제다. |
| **LOW** | **SEPARATE DEBT** | `--all-features`에서 TLS provider가 충돌한다 | crate가 `tls-aws-lc-rs`, `tls-openssl`, `tls-ring`을 상호 배타적으로 노출하면서 `--all-features`가 동시에 켠다 ([Cargo.toml](../crates/openark-rook-ceph-controller/Cargo.toml#L28-L51)). 실패는 `openark-core/src/tls.rs`에서 재현됐다. 이는 repository feature-policy debt이며 Issue #52 로직과 섞지 않는다. 별도 PR에서 각 provider matrix 또는 feature 정책으로 다룬다. |
| **TEST GAP** | **OPEN** | 실제 actuation 경로의 E2E가 없다 | unit/helper 및 Helm render 검증은 있으나 실제 block device, Kubernetes API server, built container, Rook prepare/OSD E2E가 없다. `kubectl`, container runtime, Miri도 이 환경에서 사용할 수 없었다. 현재 결과로 production safety를 주장할 수 없다. |

### C.1 고정해야 할 해석

- `/dev/disk/by-id` alias는 암호학적 identity도 reservation도 아니다.
- 열린 fd는 scan 중 identity와 signature read를 묶지만 publication 이후 살아 있지 않다.
- Rook가 현재 content signature를 다시 검사한다는 사실은 관측한 WWID, serial, `dev_t`를
  같은 물리 장치에 묶어 전달한다는 뜻이 아니다.
- Merge Patch는 list 원소를 더하는 연산이 아니다.
  controller가 기존 원소를 보존한 새 배열을 만든 뒤 배열 전체를 교체한다.
- SSA conflict 강제 해결로 atomic list를 entry별 분할 소유할 수 없다.
  Argo CD의 SSA는 `--force-conflicts`를 사용하므로 ignore boundary가 더 중요하다
  ([Argo CD sync options](https://argo-cd.readthedocs.io/en/stable/user-guide/sync-options/)).

## D. Alternatives decision matrix

| 대안 | 안전성 | 운영성 | 복잡도 | 결정 | 이유 |
|---|---|---|---|---|---|
| 현재 cluster-local agent + controller | dry-run에서는 높음, live write는 TOCTOU 미해결 | 장치 변화에 반응하고 cluster별 소유가 명확함 | 중간 | **KEEP, inert only** | 이미 구현됐고 Issue의 재사용 경계를 만족한다. 기본 비활성 및 dry-run이면 위험한 actuation 없이 계획과 증거 경로를 리뷰할 수 있다. |
| 기존 KISS workflow 확장 | host 접근 시점에는 단순함 | join 이후 장치 추가를 자동 추적하기 어렵고 workflow 재실행 의미가 불명확함 | 낮음에서 중간 | **REJECT for current branch** | 새 장치 지속 reconciliation 요구를 충분히 처리하지 못한다. 현재 두 컴포넌트를 걷어내고 다시 만드는 이득도 없다. |
| node-local partial reconciliation + durable ledger | claim과 부분 진행을 함께 설계할 수 있음 | 일부 노드 장애 격리는 좋아짐 | 높음 | **REJECT** | ledger의 수명, fencing, 복구, stale claim 정책이 새 시스템이 된다. Issue #52 초기 범위와 KISS 원칙을 넘는다. 현재 fail-closed가 더 작고 안전하다. |
| custom preparer 또는 Rook fork | actuation-time identity를 직접 묶을 가능성이 있음 | upstream 추적과 배포 운영 비용이 큼 | 매우 높음 | **REJECT** | 이 리뷰에서 새 preparer나 fork를 설계하지 않는다. production enablement의 선행 연구 후보일 뿐 현재 PR 해법이 아니다. |
| Git에 static device enumeration | review와 rollback 가시성이 높음 | 교체 및 증설마다 수동 갱신, cluster별 하드웨어 목록 중복 | 낮음 | **REJECT as Issue solution** | 작은 고정 환경의 운영 우회책은 될 수 있으나 자동 발견 요구를 해결하지 않는다. |

가장 작은 결정은 현재 구조를 삭제하거나 새 stateful subsystem을 추가하는 것이 아니다.
이미 구현된 두 컴포넌트를 비활성 및 dry-run 상태로 유지하고,
직접 실행 기본값만 안전하게 맞추는 것이다.

운영 쓰기를 당장 제공하려고 fail-closed 검사를 node-local로 약화하는 선택도 하지 않는다.
그 방식은 availability를 얻는 대신 durable claim 없이 cross-node duplicate safety를 잃는다.

## E. Recommended target architecture

### E.1 이번 PR의 target

이번 PR의 target은 production auto-enrollment가 아니라
**disabled-by-default, dry-run-by-default experimental reconciliation plumbing**이다.

구성 요소는 현재 두 개만 유지한다.

1. **Node agent DaemonSet**
   KISS `Storage` label이 있는 노드에서만 실행한다.
   `/dev/disk/by-id` 후보를 열고 fd-bound libblkid probe,
   `rdev`, sysfs, NVMe namespace, media 및 PCIe 사실을 같은 snapshot 안에서 확인한다
   ([device.rs](../crates/openark-rook-ceph-controller/src/device.rs#L19-L124),
   [device.rs](../crates/openark-rook-ceph-controller/src/device.rs#L198-L260)).
   결과는 Pod UID owner reference가 있는 ConfigMap으로 게시한다
   ([openark-rook-ceph-agent.rs](../crates/openark-rook-ceph-controller/src/bin/openark-rook-ceph-agent.rs#L40-L85)).
2. **Cluster-local controller Deployment**
   하나의 CephCluster를 대상으로 eligible Nodes, Rook discovery ConfigMaps,
   current agent Pods 및 inventories를 읽는다.
   reusable hardware class를 child policy의 semantic class로 매핑한다.
   candidate가 없으면 쓰지 않고, 증거가 불완전하거나 충돌하면 전체 reconcile을 중단한다.

### E.2 Ownership과 patch 계약

`spec.storage.nodes`와 nested `devices`는 atomic list이므로 runtime이 전체 field를 소유한다.
Git은 `useAllNodes: false`, `useAllDevices: false`와 초기 `nodes: []`를 선언하지만,
`manageStorageNodes=true`일 때 Argo는 `/spec/storage/nodes` 전체를 무시한다
([patches.yaml](../apps/rook-ceph-cluster/patches.yaml#L24-L53),
[manifest.yaml](../apps/rook-ceph-cluster/manifest.yaml#L32-L44)).

controller는 live storage를 읽고 다음 순서로 출력 배열을 만든다.

1. 기존 node 객체와 알려지지 않은 필드를 그대로 복사한다.
2. 기존 device 객체와 config를 그대로 복사한다.
3. 이미 존재하는 alias와 새 후보의 alias를 정규화해 중복을 거부한다.
4. mapping이 있는 신규 candidate만 해당 node의 `devices` 끝에 넣는다.
5. 처음 본 node에만 새 node 객체를 만든다.
6. 결과 전체를 `spec.storage.nodes`에 넣는다
   ([reconcile.rs](../crates/openark-rook-ceph-controller/src/reconcile.rs#L130-L190),
   [reconcile.rs](../crates/openark-rook-ceph-controller/src/reconcile.rs#L212-L269)).

그 뒤 CephCluster를 처음 읽었을 때의 `metadata.resourceVersion`을 patch에 넣고
JSON Merge Patch를 보낸다
([openark-rook-ceph-controller.rs](../crates/openark-rook-ceph-controller/src/bin/openark-rook-ceph-controller.rs#L589-L593),
[openark-rook-ceph-controller.rs](../crates/openark-rook-ceph-controller/src/bin/openark-rook-ceph-controller.rs#L1068-L1086)).
동시 변경이 있으면 API server precondition이 patch를 거부해야 한다.

이 Merge Patch는 element-wise additive가 아니다.
배열을 교체하되, 교체할 출력 배열을 만들 때 기존 항목과 필드를 보존하는 방식이다.
따라서 Argo와 다른 writer가 같은 field를 함께 관리하면 안 된다.

### E.3 활성화 경계

- `rookCeph.provisioning.enabled` 기본값은 `false`다.
- SmartX root Helm render/integration 경로에서는 `manageStorageNodes=true` 없이
  `enabled=true`이면 실패한다. standalone subchart는 `enabled=true`로 렌더할 수 있다.
- `manageStorageNodes=true`는 broad selector를 끄고 runtime에 list 전체를 넘기는 명시적 handoff다.
- controller `dryRun`은 Helm과 direct binary 모두 기본 `true`다.
- 현재 지원 범위는 dry-run 결과와 fail-closed 동작의 관찰까지다.
- production `dryRun=false`는 actuation-time physical identity proof와 E2E 뒤 별도 결정으로 남긴다.

durable ledger, webhook, database, custom preparer, Rook fork는 추가하지 않는다.
현재 승인 범위에서 필요하지 않고, 각자 새로운 일관성 및 운영 문제를 만든다.

## F. Code changes

### F.1 정확한 로컬 변경

기능 커밋 `4be531d3d8b45bf959d1841c4408c62050818e5f` 위의 소스 변경은 한 줄이다.

```diff
-    #[arg(long, env = "DRY_RUN", default_value_t = false)]
+    #[arg(long, env = "DRY_RUN", default_value_t = true)]
```

같은 controller binary test module에 직접 parser 테스트 하나를 추가했다.

```rust
#[test]
fn dry_run_defaults_true_without_env_or_cli_override() {
    let matches = Args::command()
        .mut_arg("dry_run", |arg| arg.env(None::<&str>))
        .try_get_matches_from([
            "openark-rook-ceph-controller",
            "--ceph-cluster-namespace",
            "rook-ceph",
            "--ceph-cluster-name",
            "rook-ceph",
            "--rook-discovery-namespace",
            "rook-ceph",
            "--provisioning-namespace",
            "openark",
            "--app-instance",
            "storage",
            "--minimum-device-bytes",
            "100",
            "--device-class-map",
            r#"{"default":"ssd"}"#,
        ])
        .unwrap();
    let args = Args::from_arg_matches(&matches).unwrap();

    assert!(args.dry_run);
}
```

이 테스트는 `dry_run` argument의 env source를 명시적으로 제거하므로 ambient `DRY_RUN`이
default assertion을 오염시킬 수 없다. 그 뒤 env와 CLI override가 없는 matches에서
`Args::from_arg_matches`로 direct invocation 기본값을 검증한다.
Helm manifest가 값을 넣는지만 확인하는 render 테스트로는 이 회귀를 잡을 수 없다.

### F.2 의도적으로 바꾸지 않은 부분

- **Identity 검사**는 그대로 둔다.
  scan 중 retarget 방어는 유효하며 제거할 이유가 없다.
  다만 그 보호 범위를 Rook actuation까지 확장해 말하지 않는다.
- **Rook matching과 prepare**는 그대로 둔다.
  upstream v1.19.8을 사용하며 custom preparer나 fork를 만들지 않는다.
- **Atomic list ownership**은 그대로 둔다.
  whole-field ignore와 exclusive runtime ownership이 CRD schema에 맞다.
- **Cluster-wide isolation 정책**은 그대로 둔다.
  durable claim 없이 partial reconciliation으로 바꾸지 않는다.
- **TLS features**는 그대로 둔다.
  Issue #52의 data path와 무관한 repository feature-policy 변경을 섞지 않는다.
- **재시도, rollback, ledger**는 추가하지 않는다.
  controller의 기존 poll loop와 API precondition 밖의 상태 시스템은 이번 판정에 필요 없다.

소스와 parser test 변경은 첫 번째 로컬 review commit으로 묶고,
이 독립 리뷰 문서는 두 번째 로컬 docs commit으로 분리한다.
이렇게 하면 안전 기본값 수정과 검토 기록을 각각 독립적으로 확인할 수 있다.

## G. Validation evidence

검증 결과는 통과, 실패, 실행하지 못한 항목을 구분한다.
전체가 green이라고 해석하면 안 된다.

### G.1 기능 브랜치 및 로컬 수정 검증

| 명령 또는 검사 | 관측 결과 | 범위 |
|---|---|---|
| `git diff --check HEAD^ HEAD` | **PASS** | 기능 커밋 diff의 whitespace 검사 |
| `cargo fmt --all -- --check` | **PASS** | workspace formatting |
| 수정 전 `cargo test --package openark-rook-ceph-controller` | **PASS, 29 tests** | 기능 커밋 package baseline |
| 수정 후 `cargo test --package openark-rook-ceph-controller` | **PASS, 30 tests** | 20 library + 10 controller binary tests, 새 parser test 포함 |
| focused `dry_run_defaults_true_without_env_or_cli_override` | **PASS** with ambient `DRY_RUN=false`; **PASS** with `DRY_RUN` unset | env source를 격리한 default parser assertion만 검증하며 production writes는 검증하지 않음 |
| `cargo clippy --package openark-rook-ceph-controller --all-targets -- -D warnings` | **PASS** | default features package lint |
| `cargo build --package openark-rook-ceph-controller --all-targets` | **PASS** | default features package build |
| `apps/rook-ceph-provisioning/tests/render.sh` | **PASS** | Helm lint, disabled render, ownership, RBAC, Argo lifecycle assertions |
| `hack/git-ci-validate.sh` | **PASS** | repository CI entry point |
| `cargo run --package openark-rook-ceph-controller --bin openark-rook-ceph-controller -- --help` | **PASS** | required options와 `--dry-run`/`DRY_RUN` surface 확인, Kubernetes contact 없음 |
| `cargo run --package openark-rook-ceph-controller --bin openark-rook-ceph-agent -- --help` | **PASS** | 예상 agent options 확인, Kubernetes contact 없음 |
| `helm template` default with `enabled=true` | **PASS** | 렌더된 controller env의 `DRY_RUN` 값이 `"true"`임을 확인 |
| `helm template` with `--set controller.dryRun=false` | **PASS** | 명시적 Helm `false` override 지원과 렌더된 값 `"false"`를 확인. template 검증일 뿐 production writes를 검증하지 않음 |

Helm render 검증은 다음 안전 경계를 실제 assertion으로 확인한다.

- disabled chart가 runtime resource를 만들지 않는다.
- SmartX root integration에서 provisioning enable에는 storage ownership opt-in이 필요하다.
- runtime-owned mode에서 declarative `storage.nodes`와 broad selector를 거부한다
  ([render.sh](../apps/rook-ceph-provisioning/tests/render.sh#L72-L109)).
- agent는 Storage node에만 배치되고 host mounts는 read-only다.
- controller는 single replica, `Recreate`, non-root로 렌더된다
  ([render.sh](../apps/rook-ceph-provisioning/tests/render.sh#L205-L316)).
- `/spec/storage/nodes` ignore와 explicit selection invariant가 렌더된다
  ([render.sh](../apps/rook-ceph-provisioning/tests/render.sh#L348-L423)).

### G.2 재현된 실패

| 명령 | 관측 결과 | 판정 |
|---|---|---|
| `cargo test --workspace --all-targets` | **FAIL**, `openark-vine-dashboard-jq` compile errors | parent에서도 재현되는 unrelated baseline failure. Issue #52 변경이 원인이 아님 |
| `cargo clippy --package openark-rook-ceph-controller --all-targets --all-features -- -D warnings` | **FAIL**, mutually exclusive TLS providers가 `openark-core/src/tls.rs`에서 충돌 | package feature-policy debt. 별도 PR 대상 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **FAIL**, TLS 충돌과 `dark-lake-api`, `openark-vine-dashboard-jq` baseline errors | workspace 전체 green 근거 없음 |

workspace test의 `openark-vine-dashboard-jq` 실패는 parent에서 같은 방식으로 재현됐다.
그러므로 현재 feature diff의 회귀로 분류하지 않지만,
실패 자체를 숨기거나 모든 check가 통과했다고 기록해서는 안 된다.

### G.3 사용할 수 없었거나 실행하지 않은 검증

| 항목 | 상태 | 영향 |
|---|---|---|
| `kubectl` client 및 API server 검증 | **NOT RUN**, `kubectl` unavailable | resourceVersion conflict와 live dry-run patch를 실제 server에서 검증하지 못함 |
| Container build/run | **NOT RUN**, container runtime unavailable | image 안의 binary, libblkid linkage, 권한과 mounts를 실행하지 못함 |
| `typos` | **NOT RUN**, tool unavailable | 전용 typo gate 없음 |
| Miri | **NOT RUN**, rustup-managed nightly unavailable | libblkid FFI에 대한 Miri 증거 없음 |
| LSP diagnostics | **NOT RUN for this worktree** | 진단 도구가 original cwd를 강제해 `/tmp` worktree를 검사할 수 없음. diagnostics pass라고 주장하지 않음 |
| 실제 block device E2E | **NOT RUN** | alias replacement, signature, `dev_t`, device replacement 경계를 검증하지 못함 |
| Rook disposable-cluster E2E | **NOT RUN** | API patch부터 prepare, ceph-volume, OSD 생성까지 검증하지 못함 |

unit test는 parser, planner, identity helper와 fixture parsing을 검증한다.
Helm test는 렌더된 manifest 문자열을 검증한다.
둘 다 실제 hardware, API server, container, Rook actuation 증거는 아니다.

## H. PR structure recommendation

### H.1 로컬 review commit 두 개

다음 순서로 두 개의 로컬 commit을 권장한다.

1. `fix(rook-ceph): default controller to dry-run`
   - controller Clap 기본값 `false`를 `true`로 변경
   - `dry_run_defaults_true_without_env_or_cli_override` 회귀 테스트 추가
   - source와 test만 포함
2. `docs(rook-ceph): add Issue 52 independent review`
   - 이 A-I 독립 리뷰 문서만 포함
   - supported scope, 실패, test gap, production block를 보존

이 분리는 기능 브랜치의 큰 구현을 다시 쪼개라는 뜻이 아니다.
현재 검토에서 새로 생긴 최소 수정과 그 검토 기록을 분리하는 방식이다.

### H.2 Feature PR에 남길 것

- agent와 controller의 experimental plumbing
- 명시적 opt-in 및 dry-run defaults
- full-list runtime ownership과 Argo ignore boundary
- child cluster의 `deviceClassMap` policy channel
- fail-closed complete-cluster evidence 검사
- 신규 장치만 대상으로 하는 additive planner
- Rook v1.19.8 pin

Rook v1.19.8은 per-device `deviceClass`의 raw-mode propagation fix를 포함하므로
feature commit에 유지해야 한다.
단순 dependency cleanup으로 떼면 기능 의미가 깨진다
([Rook PR #17407](https://github.com/rook/rook/pull/17407),
[Rook v1.19.6 release](https://github.com/rook/rook/releases/tag/v1.19.6),
[Rook v1.19.8 release](https://github.com/rook/rook/releases/tag/v1.19.8)).

### H.3 별도 future PR 또는 결정으로 남길 것

- TLS provider feature policy와 CI matrix
- unrelated workspace baseline crate fixes
- actuation-time physical identity proof 설계
- 실제 장치, API server, built image, Rook prepare/OSD E2E harness
- production `dryRun=false` enablement 결정

TLS policy를 현재 PR에 섞으면 Issue #52의 safety review와 repository-wide feature cleanup이 결합된다.
반대로 production enablement는 문서 한 줄이나 flag 변경으로 처리할 수 없다.
관측한 물리 장치와 Rook가 준비하는 물리 장치를 actuation 시점에 증명하고,
교체 및 alias retarget adversarial E2E를 통과한 뒤 별도 PR과 운영 승인을 받아야 한다.

## I. Final push recommendation

최종 판정은 목적에 따라 둘로 나눈다.

### `PUSH_FOR_REVIEW`

**조건부 승인.**
아래 조건을 모두 만족하면 현재 로컬 브랜치를 remote에 push하고 PR review를 시작해도 된다.

1. `fix(rook-ceph): default controller to dry-run` 로컬 commit이 존재한다.
2. `docs(rook-ceph): add Issue 52 independent review` 로컬 commit이 별도로 존재한다.
3. Section G의 지원되는 최종 check가 같은 내용에서 계속 통과한다.
4. 재현된 workspace 및 all-features 실패와 NOT RUN 항목을 PR 본문에서 숨기지 않는다.
5. PR scope를 disabled-by-default, dry-run-by-default experimental plumbing으로 명시한다.

이 판정은 branch commit `4be531d3d8b45bf959d1841c4408c62050818e5f`의 구현을
삭제하거나 다른 architecture로 다시 쓰기보다 `KEEP`할 근거가 충분하다는 뜻이다.
또한 local fix가 direct invocation의 즉시 위험한 기본값을 바로잡았고,
package, format, default-feature lint/build, Helm render, repository validation이 통과했다는 뜻이다.

### `DO_NOT_ENABLE_WRITES`

**운영 `dryRun=false`는 승인하지 않는다.**

현재 agent의 fd-bound probe는 scan 순간의 장치에 대해서는 의미가 있다.
하지만 열린 fd, WWID/serial/`dev_t` claim, 또는 물리 장치 reservation이
ConfigMap publication과 controller patch를 거쳐 Rook prepare로 전달되지 않는다.
Rook와 ceph-volume의 재검사는 그 시점의 path와 content를 다시 볼 뿐,
controller가 관측한 동일한 물리 장치라는 연결 증명을 제공하지 않는다.

따라서 alias retarget 또는 blank replacement TOCTOU가 남아 있다.
production enablement는 actuation-time identity proof와 real-device/API-server/container/Rook E2E가
별도 검토에서 통과한 뒤 결정해야 한다.

이번 작업에서는 remote push도 PR 생성도 수행하지 않았다.
로컬 commit 생성 여부도 이 문서가 대신하지 않는다.
권고는 두 로컬 commit과 최종 지원 check가 준비된 시점의
`PUSH_FOR_REVIEW`, 그리고 그 이후에도 유지되는 `DO_NOT_ENABLE_WRITES`다.
