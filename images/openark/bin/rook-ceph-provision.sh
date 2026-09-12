#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C LANG=C

die() { printf 'rook-ceph-provision: %s\n' "$*" >&2; exit 1; }
decimal() { [[ $1 =~ ^[0-9]+$ ]]; }
bounded_decimal() {
  local value=$1 prefix max=$2
  decimal "$value" || return 1
  prefix=${value%%[!0]*}; value=${value#"$prefix"}; [[ -n $value ]] || value=0
  ((${#value} < ${#max})) || [[ ${#value} == "${#max}" && ($value == "$max" || $value < "$max") ]] || return 1
  DECIMAL=$value
}
safe_component() { [[ $1 =~ ^[a-zA-Z0-9_.:+-]+$ && $1 != . && $1 != .. ]]; }
by_id() { [[ $1 == /dev/disk/by-id/* ]] && safe_component "${1#/dev/disk/by-id/}"; }

: "${EXPECTED_NODE_NAME:?EXPECTED_NODE_NAME is required}"
: "${DRY_RUN:?DRY_RUN is required}"
: "${MINIMUM_DEVICE_BYTES:?MINIMUM_DEVICE_BYTES is required}"
: "${OSDS_PER_DEVICE:?OSDS_PER_DEVICE is required}"
: "${DEVICE_CLASS_MAP:?DEVICE_CLASS_MAP is required}"
HOST_DEV_ROOT=${HOST_DEV_ROOT:-/host/dev}
HOST_SYS_ROOT=${HOST_SYS_ROOT:-/host/sys}
BLKID_PROBE=${BLKID_PROBE:-/usr/local/bin/rook-ceph-blkid-probe}
[[ $EXPECTED_NODE_NAME != *[[:space:]/]* ]] || die 'invalid expected node name'
[[ $DRY_RUN == true || $DRY_RUN == false ]] || die 'DRY_RUN must be true or false'
bounded_decimal "$MINIMUM_DEVICE_BYTES" 9223372036854775807 || die 'MINIMUM_DEVICE_BYTES is out of range'
MINIMUM_DEVICE_BYTES=$DECIMAL
bounded_decimal "$OSDS_PER_DEVICE" 4294967295 || die 'OSDS_PER_DEVICE is out of range'
OSDS_PER_DEVICE=$DECIMAL; [[ $OSDS_PER_DEVICE != 0 ]] || die 'OSDS_PER_DEVICE must be positive'
[[ -x $BLKID_PROBE ]] || die 'probe helper is not executable'
jq -e 'type == "object" and all(to_entries[]; (.key | length > 0) and
  (.value | type == "string" and length > 0))' <<<"$DEVICE_CLASS_MAP" >/dev/null || die 'invalid class map'

node=$(kubectl get node "$EXPECTED_NODE_NAME" -o json) || die 'cannot read expected Node'
hostname=$(jq -er --arg name "$EXPECTED_NODE_NAME" '
  select(.metadata.name == $name and .metadata.labels["node-role.kubernetes.io/kiss"] == "Storage")
  | .metadata.labels["kubernetes.io/hostname"] | select(type == "string" and length > 0)' <<<"$node") ||
  die 'Node identity or Storage role is invalid'
rook=$(kubectl get configmaps -n csi-rook-ceph \
  -l "app=rook-discover,rook.io/node=$EXPECTED_NODE_NAME" -o json) || die 'cannot read Rook discovery'
devices=$(jq -er --arg node "$EXPECTED_NODE_NAME" '
  select(.items | type == "array" and length == 1)
  | .items[0]
  | select(.metadata.namespace == "csi-rook-ceph")
  | select(.metadata.labels.app == "rook-discover" and .metadata.labels["rook.io/node"] == $node)
  | .data.devices | select(type == "string") | fromjson | select(type == "array")' <<<"$rook") ||
  die 'Rook discovery must contain exactly one valid devices payload'

jq -e '
  def path: type == "string" and test("^/dev/disk/by-id/[^/]+$");
  def words: type == "string" and (split(" ") | map(select(length > 0)) | length > 0);
  all(.[];
    . as $d | ($d.cephVolumeData | select(type == "string") | fromjson) as $c
    | ($d | type == "object") and ($d.name | type == "string" and length > 0)
    and ($d["kernel-name"] == $d.name) and ($d.devLinks | words)
    and all($d.devLinks | split(" ") | map(select(length > 0))[];
      (startswith("/dev/disk/by-id/") | not) or path)
    and ($d.size | type == "number" and . >= 0) and ($d.type == "disk")
    and ($d.rotational | type == "boolean") and ($d.readOnly | type == "boolean")
    and ($d.empty | type == "boolean") and ($c | type == "object")
    and ($c.path == ("/dev/" + $d["kernel-name"])) and ($c.available | type == "boolean")
    and ($c.rejected_reasons | type == "array")
    and all($c.rejected_reasons[]; type == "string" and test("[^[:space:]]"))
    and (($c.available and ($c.rejected_reasons | length == 0) and $d.empty and ($d.readOnly | not))
      or (($c.available | not) and ($c.rejected_reasons | length > 0)))
    and ($c.lvs | type == "array") and ($c.sys_api | type == "object")
    and ($c.sys_api.path == $c.path) and ($c.sys_api.devname == $d["kernel-name"])
    and ($c.sys_api.type == "disk") and ($c.sys_api.size == $d.size)
    and ($c.sys_api.ro == (if $d.readOnly then "1" else "0" end))
    and ($c.sys_api.rotational == (if $d.rotational then "1" else "0" end))
    and ($c.sys_api.removable | test("^[01]$")) and ($c.sys_api.partitions | type == "object"))
  and ([.[].name] | length == (unique | length))' <<<"$devices" >/dev/null || die 'malformed Rook evidence'

ceph=$(kubectl get cephcluster rook-ceph-cluster -n csi-rook-ceph -o json) || die 'cannot read CephCluster'
jq -e '
  .metadata.resourceVersion as $rv | .spec.storage as $s
  | ($rv | type == "string" and length > 0) and ($s | type == "object")
  and ($s.useAllNodes == false) and ($s.useAllDevices == false)
  and (($s.deviceFilter // "") == "") and (($s.devicePathFilter // "") == "")
  and (($s.devices // []) | type == "array" and length == 0)
  and (($s.nodes // []) | type == "array")
  and all(($s.nodes // [])[]; . as $n | ($n | type == "object")
    and ($n.name | type == "string" and length > 0)
    and ($n | has("deviceFilter") | not) and ($n | has("devicePathFilter") | not)
    and (($n.useAllDevices // false) == false) and (($n.devices // []) | type == "array")
    and all(($n.devices // [])[]; . as $d | ($d | type == "object")
      and ($d.name | type == "string" and length > 0)))
  and ([($s.nodes // [])[].name] | length == (unique | length))' <<<"$ceph" >/dev/null ||
  die 'CephCluster storage is not safe for explicit append-only management'

declare -A claims existing_here selected ranks
while IFS= read -r row; do
  jq -e '.[0] | type == "string" and length > 0' <<<"$row" >/dev/null || die 'invalid Ceph node name'
  jq -e '.[1] | type == "string" and
    (test("^[a-zA-Z0-9_.:+-]+$") or test("^/dev/[a-zA-Z0-9_.:+-]+$") or
     test("^/dev/disk/by-id/[a-zA-Z0-9_.:+-]+$")) and
    . != "." and . != ".." and . != "/dev/disk/by-id/." and . != "/dev/disk/by-id/.."' \
    <<<"$row" >/dev/null || die 'invalid registered device identity'
  owner=$(jq -r '.[0]' <<<"$row"); identity=$(jq -r '.[1]' <<<"$row")
  if by_id "$identity"; then normalized=$identity
  elif [[ $identity == /dev/* ]] && safe_component "${identity#/dev/}"; then normalized=$identity
  elif safe_component "$identity"; then normalized=/dev/$identity
  else die 'invalid registered device identity'
  fi
  if [[ ${claims[$normalized]+yes} ]]; then
    die "duplicate device identity $normalized"
  fi
  claims[$normalized]=$owner
  [[ $owner == "$hostname" ]] && existing_here[$normalized]=1
done < <(jq -c '.spec.storage.nodes // [] | .[] | .name as $n | (.devices // [])[] | [$n,.name]' \
  <<<"$ceph")

shopt -s nullglob
declare -a alias_kernels=() alias_paths=()
for alias in "$HOST_DEV_ROOT"/disk/by-id/*; do
  name=${alias##*/}
  case $name in
    wwn-*) rank=0 ;;
    scsi-*) rank=1 ;;
    ata-*) rank=2 ;;
    nvme-eui.*|nvme-uuid.*|nvme-*_*) rank=3 ;;
    *) continue ;;
  esac
  safe_component "$name" && by_id "/dev/disk/by-id/$name" || die "unsafe stable alias $name"
  [[ $name =~ -part[0-9]+$ ]] && continue
  target=$(readlink -f "$alias") || die "cannot resolve $name"
  [[ $target == "$HOST_DEV_ROOT"/* && ${target#"$HOST_DEV_ROOT"/} != */* ]] || die "unsafe alias $name"
  kernel=${target##*/}
  key=$kernel
  alias_kernels+=("$kernel"); alias_paths+=("/dev/disk/by-id/$name")
  if [[ ! ${selected[$key]+yes} || $rank -lt ${ranks[$key]} ||
    ($rank -eq ${ranks[$key]} && $name < ${selected[$key]##*/}) ]]; then
    selected[$key]=$alias
    ranks[$key]=$rank
  fi
done

verify_identity() {
  local alias=$1 fd=$2 target=$3 kernel=$4 expected=$5 actual raw major minor sys_major sys_minor
  [[ $(stat -Lc %F "$alias") == 'block special file' ]] || return 1
  [[ $(readlink -f "$alias") == "$target" && $(readlink -f "/proc/$$/fd/$fd") == "$target" ]] || return 1
  actual=$(stat -Lc %t:%T "$alias")
  [[ $actual == "$expected" && $(stat -Lc %t:%T "/proc/$$/fd/$fd") == "$expected" ]] || return 1
  raw=$(<"$HOST_SYS_ROOT/class/block/$kernel/dev")
  IFS=: read -r major minor <<<"$expected"; IFS=: read -r sys_major sys_minor <<<"$raw"
  [[ $major =~ ^[[:xdigit:]]{1,8}$ && $minor =~ ^[[:xdigit:]]{1,8}$ ]] || return 1
  bounded_decimal "$sys_major" 4294967295 || return 1; sys_major=$DECIMAL
  bounded_decimal "$sys_minor" 4294967295 || return 1; sys_minor=$DECIMAL
  ((16#$major == sys_major && 16#$minor == sys_minor)) || return 1
}

declare -a held_alias held_fd held_target held_kernel held_devt
additions='[]'
kernels=(); ((${#alias_paths[@]} == 0)) || mapfile -t kernels < <(printf '%s\n' "${!selected[@]}" | sort)
for kernel in "${kernels[@]}"; do
  alias=${selected[$kernel]}; path=/dev/disk/by-id/${alias##*/}
  [[ ! -e $HOST_SYS_ROOT/class/block/$kernel/partition ]] || continue
  [[ $(stat -Lc %F "$alias") == 'block special file' ]] || continue
  registered=false
  identities=("/dev/$kernel" "$kernel")
  for index in "${!alias_paths[@]}"; do
    [[ ${alias_kernels[$index]} == "$kernel" ]] && identities+=("${alias_paths[$index]}")
  done
  for identity in "${identities[@]}"; do
    if [[ ${claims[$identity]+yes} && ${claims[$identity]} != "$hostname" ]]; then
      die "device identity $identity is claimed by another node"
    fi
    [[ ${existing_here[$identity]+yes} ]] && registered=true
  done
  $registered && continue
  rook_device=$(jq -ec --arg kernel "$kernel" '[.[] | select(.["kernel-name"] == $kernel)]
    | if length == 0 then empty elif length == 1 then .[0] else error("ambiguous") end' <<<"$devices") ||
    die "ambiguous Rook evidence for $kernel"
  [[ -n ${rook_device:-} ]] || continue
  jq -e --arg path "$path" '.devLinks | split(" ") | index($path) != null' <<<"$rook_device" >/dev/null ||
    die "Rook evidence does not contain selected alias $path"
  sys=$HOST_SYS_ROOT/class/block/$kernel
  read -r sectors <"$sys/size" || die "cannot read size for $kernel"
  read -r ro <"$sys/ro" || die "cannot read read-only state for $kernel"
  read -r removable <"$sys/removable" || die "cannot read removable state for $kernel"
  read -r rotational <"$sys/queue/rotational" || die "cannot read medium for $kernel"
  bounded_decimal "$sectors" 18014398509481983 || die 'invalid sysfs sectors'; sectors=$DECIMAL
  bounded_decimal "$ro" 1 || die 'invalid sysfs read-only state'; ro=$DECIMAL
  bounded_decimal "$removable" 1 || die 'invalid sysfs removable state'; removable=$DECIMAL
  bounded_decimal "$rotational" 1 || die 'invalid sysfs rotational state'; rotational=$DECIMAL
  bytes=$((sectors * 512))
  ((bytes >= MINIMUM_DEVICE_BYTES)) || continue
  ((ro == 0 && removable == 0)) || continue
  jq -e --argjson size "$bytes" --argjson ro "$ro" --argjson rem "$removable" \
    --argjson rot "$rotational" '.size == $size and .readOnly == ($ro != 0)
      and .rotational == ($rot != 0) and (.cephVolumeData | fromjson | .sys_api.removable == ($rem|tostring))' \
    <<<"$rook_device" >/dev/null || die "Rook and sysfs disagree for $kernel"
  jq -e '.cephVolumeData | fromjson | .available == true' <<<"$rook_device" >/dev/null || continue
  exec {fd}<"$alias" || die "cannot open $path"
  target=$(readlink -f "$alias"); devt=$(stat -Lc %t:%T "$alias")
  verify_identity "$alias" "$fd" "$target" "$kernel" "$devt" || die "device identity changed for $path"
  held_alias+=("$alias"); held_fd+=("$fd"); held_target+=("$target")
  held_kernel+=("$kernel"); held_devt+=("$devt")
  set +e; "$BLKID_PROBE" "$fd"; probe_status=$?; set -e
  case $probe_status in 0|254) continue ;; 1) ;; *) die "signature probe failed for $path" ;; esac
  verify_identity "$alias" "$fd" "$target" "$kernel" "$devt" || die "device identity changed for $path"
  hardware=; medium=ssd
  ((rotational != 0)) && medium=hdd
  if [[ $kernel == nvme* ]]; then
    medium=nvme
    [[ -r $sys/nsid ]] || die "missing NVMe NSID for $kernel"
    read -r nsid <"$sys/nsid" || die "cannot read NVMe NSID for $kernel"
    bounded_decimal "$nsid" 4294967294 || die "invalid NVMe NSID for $kernel"
    nsid=$DECIMAL; [[ $nsid != 0 ]] || die "invalid NVMe NSID for $kernel"
    if device=$(readlink -f "$sys/device" 2>/dev/null); then
      bdf=
      IFS=/ read -ra parts <<<"$device"
      for part in "${parts[@]}"; do
        [[ $part =~ ^[[:xdigit:]]{4}:[[:xdigit:]]{2}:[[:xdigit:]]{2}\.[[:xdigit:]]$ ]] && bdf=$part
      done
      if [[ -n $bdf ]]; then
        pci=${device%/$bdf*}/$bdf
        read -r speed _ <"$pci/current_link_speed" || die "cannot read PCIe speed for $kernel"
        read -r width <"$pci/current_link_width" || die "cannot read PCIe width for $kernel"
        aggregate=$(jq -ner --arg speed "$speed" --arg width "$width" '
          ($speed|tonumber) as $s | ($width|tonumber) as $w | select($s > 0 and $w > 0) | $s * $w') ||
          die "invalid PCIe link for $kernel"
        hardware=nvme-pcie-${aggregate//./p}gt
      fi
    fi
  fi
  device_class=$(jq -er --arg hardware "$hardware" --arg medium "$medium" '
    if $hardware != "" and has($hardware) then .[$hardware] elif has($medium) then .[$medium] else empty end' \
    <<<"$DEVICE_CLASS_MAP") || continue
  additions=$(jq -c --arg path "$path" --arg class "$device_class" --arg osds "$OSDS_PER_DEVICE" \
    '. + [{name:$path,config:{deviceClass:$class,osdsPerDevice:$osds}}]' <<<"$additions")
done

(( $(jq 'length' <<<"$additions") > 0 )) || exit 0
patch=$(jq -c --arg host "$hostname" --argjson additions "$additions" '
  . as $cluster | ($cluster.spec.storage.nodes // []) as $nodes
  | ($nodes | map(.name) | index($host)) as $index
  | (if $index == null then $nodes + [{name:$host,devices:$additions}]
     else $nodes | .[$index].devices = ((.[$index].devices // []) + $additions) end) as $planned
  | {metadata:{resourceVersion:$cluster.metadata.resourceVersion},spec:{storage:{nodes:$planned}}}' <<<"$ceph")
for index in "${!held_fd[@]}"; do
  verify_identity "${held_alias[$index]}" "${held_fd[$index]}" "${held_target[$index]}" \
    "${held_kernel[$index]}" "${held_devt[$index]}" || die 'device identity changed before patch'
done
args=(patch cephcluster rook-ceph-cluster -n csi-rook-ceph --type=merge -p "$patch")
[[ $DRY_RUN == true ]] && args+=(--dry-run=server)
kubectl "${args[@]}" >/dev/null || die 'CephCluster patch failed'
