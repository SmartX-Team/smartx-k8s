#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C LANG=C KUBECONFIG=/dev/null
NM='{"nvme-pcie-32gt":"direct","nvme":"flash"}' SM='{"ssd":"fast"}' F=fixture V=nvme Z=no_patch P=patch
ROOT=${ROOT_OVERRIDE:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)}
APP="$ROOT/apps/rook-ceph-provisioning" SCRIPT="$ROOT/images/openark/bin/rook-ceph-provision.sh"
PROBE_SRC="$ROOT/images/openark/src/rook-ceph-blkid-probe.c" NP=/dev/disk/by-id/nvme-eui.new M='{"nvme":"flash"}'
HN=18446744073709551617 TMP=$(mktemp -d) RK='Rook ' NS='NVMe NSID ' S=PROBE_STATUS I=MINIMUM O=role
STATE="$TMP/state" H=$TMP/probe-real K='node-role.kubernetes.io/kiss' RL='relative rejected' E=' excluded' Q=' rejected'
mkdir -p "$STATE" "$TMP/bin"
R="$STATE/rook.json" C="$STATE/ceph.json" N="$STATE/node.json" L="$STATE/calls" D='alias and FD identity change'
cleanup() {
  jobs -pr | xargs -r kill 2>/dev/null || true
  rm -rf "$TMP"
}
fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
pass() { CASES=$((${CASES:-0} + 1)); printf 'PASS: %s\n' "$*"; }
trap cleanup EXIT HUP INT TERM
cat >"$TMP/bin/kubectl" <<'PY'
#!/usr/bin/env python3
import json, os, pathlib, subprocess, sys
a, state, parent = sys.argv[1:], pathlib.Path(os.environ["STATE"]), os.getppid()
assert os.environ.get("KUBECONFIG") == "/dev/null" and not any("://" in x for x in a)
node = ["get", "node", "node-a", "-o", "json"]
rook = ["get", "configmaps", "-n", "csi-rook-ceph", "-l",
        "app=rook-discover,rook.io/node=node-a", "-o", "json"]
ceph = ["get", "cephcluster", "rook-ceph-cluster", "-n", "csi-rook-ceph", "-o", "json"]
op, rv = "", "-"
if a == node:
    op, source = "node", "node.json"
elif a == rook:
    op, source = "rook", "rook.json"
elif a == ceph:
    op, source = "ceph", "ceph.json"
    rv = json.loads((state / source).read_text())["metadata"]["resourceVersion"]
elif a[:5]==["patch","cephcluster","rook-ceph-cluster","-n","csi-rook-ceph"]:
    op = "patch"
    dry = "--dry-run=server" in a
    assert dry == (os.environ.get("EXPECT_DRY", "true") == "true") and len(a) == 8 + dry
    flags = [x for x in a if x in ("-p", "--patch")]
    assert len(flags) == 1 and a.count("--type=merge") == 1
    body = json.loads(a[a.index(flags[0]) + 1])
    rv = body["metadata"]["resourceVersion"]
    (state/"calls").open("a").write(f"{parent}|{op}|{rv}\n")
    rec = (state / "fd").read_text().rstrip("\n").split("|")
    assert int(rec[0]) == parent
    fd, alias = f"/proc/{parent}/fd/{rec[1]}", os.environ["HOST_DEV_ROOT"] + "/disk/by-id/" + rec[2]
    stat = lambda p: subprocess.check_output(["stat", "-Lc", "%t:%T", p], text=True).strip()
    assert os.path.exists(fd) and os.path.realpath(fd) == rec[3] and stat(fd) == rec[4]
    assert stat(alias) == rec[4] and pathlib.Path(os.environ["HOST_SYS_ROOT"] + rec[5]).read_text().strip() == rec[6]
    (state / "patch.json").write_text(json.dumps(body, separators=(",", ":")))
    if os.environ.get("CONFLICT") == "true" and not (state / "conflicted").exists():
        (state / "conflicted").touch()
        print("patched\nConflict (409)", file=sys.stderr)
        sys.exit(1)
    print('{"status":"Success"}')
else:
    raise SystemExit("unexpected kubectl argv: " + repr(a))
if op != "patch":
    (state / "calls").open("a").write(f"{parent}|{op}|{rv}\n")
if op in {"node", "rook", "ceph"}:
    print((state / source).read_text())
PY
cat >"$TMP/bin/stat" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ ${STAT_KIND:-block} != block ]]; then echo 'regular file'; exit; fi
name=$(basename "$(readlink -f "${!#}")")
case "$*" in
  *%F*) echo 'block special file' ;;
  *%t:%T*) case $name in nvme*) echo 103:0;; sdb) echo 8:1;; *) echo 8:0;; esac ;;
  *) exec /usr/bin/stat "$@" ;;
esac
SH
cat >"$TMP/bin/probe" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
fd=${1-}
[[ $fd =~ ^[0-9]+$ && -e /proc/self/fd/$fd ]] || exit 255
by=$HOST_DEV_ROOT/disk/by-id alias=wwn-new
[[ -e $by/nvme-eui.new ]] && alias=nvme-eui.new
target=$(readlink -f "/proc/$PPID/fd/$fd") dev=$(stat -Lc %t:%T "/proc/$PPID/fd/$fd")
kernel=${target##*/} sys="/class/block/${target##*/}/dev"
raw=$(<"$HOST_SYS_ROOT$sys")
printf '%s|%s|%s|%s|%s|%s|%s\n' "$PPID" "$fd" "$alias" "$target" "$dev" "$sys" "${raw%$'\n'}" >"$STATE/fd"
printf '%s|probe|-\n' "$PPID" >>"$STATE/calls"
if [[ ${PROBE_STATUS:-1} == retarget ]]; then ln -sfn ../../sdb "$by/wwn-new"; exit 1; fi
exit "${PROBE_STATUS:-1}"
SH
chmod +x "$TMP/bin/"{kubectl,stat,probe}
export PATH="$TMP/bin:$PATH" STATE
fixture() {
  D=$TMP/dev B=$TMP/sys/class/block b=$TMP/sys/class/block/sda P="$STATE/patch.json" L="$STATE/calls"
  rm -rf "$D" "$TMP/sys"
  rm -f "$STATE"/{calls,patch.json,conflicted,fd}
  mkdir -p "$D/disk/by-id" "$b/queue"
  truncate -s 1M "$D/"{sda,sdb}
  for alias in ata-z scsi-z wwn-new; do ln -s ../../sda "$D/disk/by-id/$alias"; done
  printf '8:0\n' >"$b/dev" && printf '2048\n' >"$b/size"
  for field in ro removable; do echo 0 >"$b/$field"; done
  echo 1 >"$b/queue/rotational"
  jq -cn '{metadata:{name:"node-a",labels:{"node-role.kubernetes.io/kiss":"Storage",
    "kubernetes.io/hostname":"host-a"}}}' >"$N"
  cv=$(jq -cn '{path:"/dev/sda",available:true,rejected_reasons:[],lvs:[],sys_api:{path:"/dev/sda",
    devname:"sda",type:"disk",size:1048576,ro:"0",removable:"0",rotational:"1",partitions:{},id_bus:""}}')
  d=$(jq -cn --arg c "$cv" '{name:"sda","kernel-name":"sda",devLinks:
    "/dev/disk/by-id/wwn-new /dev/disk/by-id/scsi-z",size:1048576,type:"disk",rotational:true,
    readOnly:false,empty:true,cephVolumeData:$c}')
  jq -cn --arg d "[$d]" '{items:[{metadata:{name:"rook-discover-a",namespace:"csi-rook-ceph",
    labels:{app:"rook-discover","rook.io/node":"node-a"}},data:{devices:$d}}]}' >"$R"
  jq -cn '{metadata:{name:"rook-ceph-cluster",resourceVersion:"7"},spec:{storage:{useAllNodes:false,
    useAllDevices:false,unknown:"keep",nodes:[{name:"host-a",location:"rack=a",devices:[{name:
    "/dev/disk/by-id/wwn-old",config:{deviceClass:"old"},extra:[1,2]}]}]}}}' >"$C"
}
edit() { jq "$1" "$2" >"$STATE/x" && mv "$STATE/x" "$2"; }
nvme() {
  fixture
  mv "$D/sda" "$D/nvme0n1" && mv "$B/sda" "$B/nvme0n1"
  rm "$D/disk/by-id/"* && ln -s ../../nvme0n1 "$D/disk/by-id/nvme-eui.new"
  b=$B/nvme0n1
  printf '259:0\n' >"$b/dev" && printf '0\n' >"$b/queue/rotational" && printf '1\n' >"$b/nsid"
  pci="$TMP/sys/devices/pci0000:00/0000:01:00.0"
  mkdir -p "$pci" && printf '8 GT/s\n' >"$pci/current_link_speed"
  printf '4\n' >"$pci/current_link_width" && ln -s "$pci" "$b/device"
  edit '.items[0].data.devices|=(fromjson|.[0]|=(.name="nvme0n1"|.["kernel-name"]="nvme0n1"|
    .devLinks="/dev/disk/by-id/nvme-eui.new"|.rotational=false|.cephVolumeData|=(fromjson|
    .path="/dev/nvme0n1"|.sys_api.path="/dev/nvme0n1"|.sys_api.devname="nvme0n1"|
    .sys_api.rotational="0"|tojson))|tojson)' "$R"
}
invoke() {
  set +e
  OUTPUT=$(HOST_DEV_ROOT="$TMP/dev" HOST_SYS_ROOT="$TMP/sys" BLKID_PROBE="$TMP/bin/probe" \
    EXPECTED_NODE_NAME=node-a DRY_RUN="${DRY_RUN:-true}" MINIMUM_DEVICE_BYTES="${MINIMUM:-0}" \
    OSDS_PER_DEVICE="${OSDS:-1}" DEVICE_CLASS_MAP="${MAP:-{\"hdd\":\"capacity\"\}}" "$SCRIPT" 2>&1)
  STATUS=$?
  set -e
}
no_patch() {
  [[ $STATUS == "$2" && ! -e $P ]] && ! grep -q '|patch|' "$L" 2>/dev/null || fail "$1 ($STATUS)"
  pass "$1"
}
patch() {
  [[ $STATUS == 0 && -s $STATE/patch.json ]] || fail "$1 ($STATUS): $OUTPUT"
  want=$(jq -c --arg c "$2" --arg p "${3:-/dev/disk/by-id/wwn-new}" '.spec.storage.nodes[0].devices
    +=[{name:$p,config:{deviceClass:$c,osdsPerDevice:"1"}}]|{metadata:{resourceVersion:.metadata.resourceVersion},
    spec:{storage:{nodes:.spec.storage.nodes}}}' "$STATE/ceph.json")
  jq -e --argjson w "$want" '.==$w' "$STATE/patch.json" >/dev/null || fail "$1 body"
  pass "$1"
}
printf '%s\n' '#include <stdlib.h>' \
  'int __wrap_blkid_do_safeprobe(void *p){(void)p;return atoi(getenv("SAFE_RESULT"));}' >"$TMP/wrap.c"
mkdir -p "$TMP/yaml/"{templates,files} && printf 'apiVersion: v2\nname: yaml\nversion: 1\n' >"$TMP/yaml/Chart.yaml"
cat >"$TMP/yaml/templates/parse.yaml" <<'TPL'
{{- $raw := .Files.Get "files/input.yaml" -}}{{- if .Values.render }}{{- $raw = tpl $raw . -}}{{- end -}}
{{- $out := list -}}{{- range regexSplit "(?m)^---[[:space:]]*$" $raw -1 -}}
{{- if trim . }}{{- $x := fromYaml (printf "value:\n%s" (indent 2 .)) -}}{{- $out = append $out $x.value -}}{{- end -}}
{{- end -}}{{- $json := $out | toJson -}}{"apiVersion":"v1","kind":"ConfigMap","data":{"json":{{ $json | quote }}}}
TPL
cat >"$TMP/verify.py" <<'PY'
import collections, json, pathlib, re, shlex, subprocess, sys
enabled, kiss, main, chart, image, cmap, calls, root = sys.argv[1:]
def parse(text,*helm_args):
 pathlib.Path(chart,"files/input.yaml").write_text(text)
 out=subprocess.check_output(["helm","template","yaml",chart,*helm_args],text=True)
 return json.loads(json.loads(next(x for x in out.splitlines() if x.startswith("{")))["data"]["json"])
def one_document(text,*helm_args):
 docs=parse(text,*helm_args)
 assert len(docs)==1
 return docs[0]
docs=parse(open(enabled).read())
ids=[(x["kind"],x["metadata"]["name"],x["metadata"].get("namespace")) for x in docs]
assert len(docs)==len(ids)==len(set(ids))==6
by,ns,name=dict(zip(ids,docs)),"csi-rook-ceph-provisioning","rook-ceph-provisioning"
keys={("ConfigMap",name,ns),("ServiceAccount",name,ns),("Role",name,"csi-rook-ceph"),
      ("RoleBinding",name,"csi-rook-ceph"),("ClusterRole",name,None),("ClusterRoleBinding",name,None)}
cm=by["ConfigMap",name,ns]
assert set(by)==keys
assert cm["apiVersion"]=="v1" and cm["data"]=={"DRY_RUN":"true","MINIMUM_DEVICE_BYTES":"0",
 "OSDS_PER_DEVICE":"1","DEVICE_CLASS_MAP":cmap,"IMAGE":image,"IMAGE_PULL_POLICY":"Always"}
assert by["Role",name,"csi-rook-ceph"]["rules"]==[{
 "apiGroups":[""],"resources":["configmaps"],"verbs":["get","list"]},
 {"apiGroups":["ceph.rook.io"],"resources":["cephclusters"],"resourceNames":["rook-ceph-cluster"],
  "verbs":["get","patch"]}]
assert by["ClusterRole",name,None]["rules"]==[{"apiGroups":[""],"resources":["nodes"],"verbs":["get"]}]
for kind,scope in (("RoleBinding","csi-rook-ceph"),("ClusterRoleBinding",None)):
 x=by[kind,name,scope]
 ref={"apiGroup":"rbac.authorization.k8s.io","kind":kind[:-7],"name":name}
 assert x["roleRef"]==ref and x["subjects"]==[{"kind":"ServiceAccount","name":name,"namespace":ns}]
plays=one_document(open(kiss).read())
assert len(plays)==1 and set(plays[0])=={"hosts","tasks"}
play,top=plays[0],plays[0]["tasks"][0]
assert play["hosts"]=="target" and len(play["tasks"])==1
policy,flow=top["block"]
command=lambda t: t.get("command",t.get("shell",""))
assert set(top)=={"name","when","delegate_to","block"} and len(top["block"])==2
assert (top["when"],top["delegate_to"])==("kiss_group_role == 'Storage'",
 "{{ groups['kube_control_plane'] | first }}")
assert set(flow)=={"name","when","block","always"} and len(flow["block"])==3 and len(flow["always"])==3
def words(t,job="JOB"):
 s=command(t).replace("{{ bin_dir }}/kubectl","kubectl").replace("{{ provision_job.stdout }}",job)
 return shlex.split(" ".join(s.split()))
assert words(policy)==["kubectl","get","configmap",name,"-n",ns,"--ignore-not-found","-o","json"]
assert policy["register"]=="provision_policy" and flow["when"]=="provision_policy.stdout != ''"
create,wait,failed=flow["block"]
logs,status,delete=flow["always"]
assert [words(x) for x in (create,wait,logs,status,delete)]==[
 ["kubectl","create","-n",ns,"-f","-","-o","jsonpath={.metadata.name}"],
 ["kubectl","wait","-n",ns,"--for=condition=complete","job/JOB","--timeout=10m"],
 ["kubectl","logs","-n",ns,"job/JOB"], ["kubectl","get","job","JOB","-n",ns,"-o","json"],
 ["kubectl","delete","job","JOB","-n",ns,"--ignore-not-found"]]
assert create["register"]=="provision_job" and wait["register"]=="provision_wait" and wait["failed_when"] is False
assert failed["fail"]["msg"] and failed["when"]=="provision_wait.rc != 0"
job=one_document(re.sub(r'{{.*?}}','VALUE',create["args"]["stdin"]))
assert job["metadata"]["generateName"]=="rook-ceph-provision-" and job["spec"]["backoffLimit"]==1
pod=job["spec"]["template"]["spec"]
c=pod["containers"][0]
assert c["command"]==["/usr/local/bin/rook-ceph-provision"],f"H1 Job command: {c.get('command')}"
root_yaml=subprocess.check_output(["helm","template","smartx",root,"--set-string",
 "rookCeph.provisioning.minimumDeviceBytes=0010","--set-string",
 "rookCeph.provisioning.osdsPerDevice=0010"],text=True,stderr=subprocess.DEVNULL)
application=next(x for x in parse(root_yaml)
 if x and x.get("metadata",{}).get("name")=="smartx-rook-ceph-provisioning")
sources=application["spec"]["sources"]
provision=next(x for x in sources if x.get("path")=="apps/rook-ceph-provisioning")
values=provision["helm"]["valuesObject"]
expected={"deviceClassMap":{},"dryRun":True,"enabled":False,
 "image":{"repo":"quay.io/ulagbulag/openark","tag":""},"manageStorageNodes":False,
 "minimumDeviceBytes":"0010","osdsPerDevice":"0010"}
manifest=one_document(pathlib.Path(root,"apps/rook-ceph-provisioning/manifest.yaml").read_text())
assert manifest["spec"]["app"]["patched"] is False,"H2 manifest patched is not false"
assert "valueFiles" not in provision["helm"],f"H2 valueFiles present: {provision['helm'].get('valueFiles')}"
assert not any(x.get("ref")=="cluster" for x in sources),"H2 cluster ref present"
assert values==expected,f"H2 valuesObject changed: {values!r}"
child_yaml=subprocess.check_output(["helm","template","p",str(pathlib.Path(root,"apps/rook-ceph-provisioning")),
 "--set","enabled=true,manageStorageNodes=true,image.repo=r,image.tag=v","--set-string","minimumDeviceBytes=0010",
 "--set-string","osdsPerDevice=0010"],text=True)
child=next(x for x in parse(child_yaml) if x and x.get("kind")=="ConfigMap")
data=child["data"]
actual=values["minimumDeviceBytes"],values["osdsPerDevice"],data["MINIMUM_DEVICE_BYTES"],data["OSDS_PER_DEVICE"]
assert actual==("0010","0010","0010","0010"),f"H4 leading zeros changed: {actual!r}"
assert pod["nodeName"]=="VALUE" and pod["serviceAccountName"]==name and pod["restartPolicy"]=="Never"
assert c["securityContext"]=={"privileged":True,"runAsUser":0} and c["envFrom"]==[
 {"configMapRef":{"name":name}}] and {x["mountPath"] for x in c["volumeMounts"] if x["readOnly"]}=={
 "/host/dev","/host/sys"}
assert c["image"]=="VALUE" and c["env"]==[{"name":"EXPECTED_NODE_NAME","value":"VALUE"}]
assert {x["hostPath"]["path"] for x in pod["volumes"]}=={"/dev","/sys"}
def kubectl(argv,present,wait_rc,serial,log):
 op="policy" if argv[1:3]==["get","configmap"] else argv[1]
 log.append((op,argv))
 return {"policy":(0,"{}" if present else ""),"create":(0,f"job-{serial}"),"wait":(wait_rc,"")}.get(op,(0,""))
def run(present,wait_rc,serial):
 log=[]
 _,out=kubectl(words(policy),present,wait_rc,serial,log)
 if flow["when"]=="provision_policy.stdout != ''" and not out:return 0,log,None
 _,job=kubectl(words(create),present,wait_rc,serial,log)
 rc,_=kubectl(words(wait,job),present,wait_rc,serial,log)
 [kubectl(words(t,job),present,wait_rc,serial,log) for t in flow["always"]]
 return int(failed["when"]=="provision_wait.rc != 0" and rc!=0),log,job
assert [x[0] for x in run(False,0,0)[1]]==["policy"]
bad,good=run(True,1,1),run(True,0,2)
sequence=["policy","create","wait","logs","get","delete"]
assert bad[0]==1 and good[0]==0 and [x[0] for x in bad[1]]==[x[0] for x in good[1]]==sequence and bad[2]!=good[2]
imports=one_document(pathlib.Path(main).read_text())
assert [x["import_playbook"] for x in imports]==["./init-openark-vine.yaml","./add-node-as-worker.yaml",
 "./add-node-role.yaml","./add-node-labels.yaml","./provision-rook-ceph.yaml"]
groups=collections.defaultdict(list)
for line in open(calls):
 p,op,rv=line.strip().split('|')
 groups[p].append((op,rv))
assert len(groups)==2 and sorted(groups.values())==sorted([
 [('node','-'),('rook','-'),('ceph','7'),('probe','-'),('patch','7')],
 [('node','-'),('rook','-'),('ceph','8'),('probe','-'),('patch','8')]])
apps,render=pathlib.Path(main).parents[3],"render=true,rookCeph.provisioning.manageStorageNodes=true"
cluster=one_document((apps/"rook-ceph-cluster/manifest.yaml").read_text(),"--set",render)["spec"]
operator=one_document((apps/"rook-ceph-operator/manifest.yaml").read_text())["spec"]
source={"repoUrl":"https://charts.rook.io/release","version":"v1.19.8"}
for x,c in ((cluster,"rook-ceph-cluster"),(operator,"rook-ceph")):
 assert x["source"]=={**source,"chart":c}
assert cluster["app"]["ignoreDifferences"]==[{"group":"ceph.rook.io","kind":"CephCluster",
 "jsonPointers":["/spec/storage/nodes"]}]
assert cluster["app"]["sync"]["respectIgnoreDifferences"] is True
PY
enabled="$TMP/e" fallback="$TMP/f" custom="$TMP/c"; ! helm template p "$APP" | grep -q '^kind:' || fail disabled
helm template p "$APP" --set enabled=true,manageStorageNodes=true,image.repo=registry/openark,image.tag=v1 >"$enabled"
helm template p "$APP" --set enabled=true,manageStorageNodes=true,image.repo=registry/openark >"$fallback"
helm template p "$APP" --set enabled=true,manageStorageNodes=true,image.repo=r,image.tag=v \
  --set-json 'deviceClassMap={"nvme":"fast"}' >"$custom"
! helm template p "$APP" --set enabled=true >/dev/null 2>&1 || fail 'ownership guard'
verify() { python3 "$TMP/verify.py" "$1" "$ROOT/apps/openark-kiss/tasks/join/provision-rook-ceph.yaml" \
  "$ROOT/apps/openark-kiss/tasks/join/main-worker.yaml" "$TMP/yaml" "$2" "$3" "$L" "$ROOT"; }
[[ -x $SCRIPT ]] || fail 'missing scanner'
cc -Wall -Wextra -Werror "$PROBE_SRC" -lblkid -o "$TMP/probe-real"
cc -Wall -Wextra -Werror "$PROBE_SRC" "$TMP/wrap.c" -Wl,--wrap=blkid_do_safeprobe -lblkid -o "$TMP/probe-map"
truncate -s 4M "$TMP/a"; exec {fd}<>"$TMP/a"; mv "$TMP/a" "$TMP/held"; truncate -s 4M "$TMP/a"
mkswap -f /proc/$$/fd/$fd >/dev/null; "$H" $fd; exec {blank}<>"$TMP/a"; for pair in 1:1 0:0 -2:254 -1:255; do
  set +e; SAFE_RESULT=${pair%:*} "$TMP/probe-map" "$blank"; got=$?; set -e
  [[ $got == "${pair#*:}" ]] || fail "helper mapping $pair got $got"
done; set +e; "$TMP/probe-real" "$blank"; got=$?; set -e; [[ $got == 1 ]] || fail 'real blank status'
exec {fd}>&- {blank}>&-; pass 'real helper blank, signed, ambivalent, and error mapping'
conflict() {
  [[ $STATUS == 1 && -s $STATE/patch.json ]] || fail 409
  pass "$1"
}
scenario() {
  "$device"
  export "${env1:-CASE_ENV=}" "${env2:-CASE_ENV=}"
  case $action in
    -) ;;
    w) printf '%s\n' "$value" >"$b/$argument" ;;
    t) touch "$b/$argument" ;;
    r) rm "$b/$argument" ;;
    a) rm -f "$D/disk/by-id/"* ;;
    e) edit "$value" "${!argument}" ;;
    q) edit ".items[0].data.devices|=(fromjson|$value|tojson)" "$R" ;;
    u) edit '.items[0].data.devices|=(fromjson|.[0].cephVolumeData|=(fromjson|.available=false|
      .rejected_reasons=["LVM"]|tojson)|tojson)' "$R" ;;
    g) edit '.spec.storage.nodes[0].devices[0].name="/dev/disk/by-id/wwn-new"' "$C" ;;
    x) edit '.spec.storage.nodes += [{name:"other",devices:[{name:"/dev/disk/by-id/wwn-new"}]}]' "$C" ;;
    s) printf '0\n' >"$b/queue/rotational"
      edit '.items[0].data.devices|=(fromjson|.[0]|=(.rotational=false|.cephVolumeData|=(fromjson|
        .sys_api.rotational="0"|tojson))|tojson)' "$R" ;;
    l) ln -s ../../sda "$D/disk/by-id/"$'wwn-a\tb'
      edit '.items[0].data.devices|=(fromjson|.[0].devLinks="/dev/disk/by-id/wwn-a\tb"|tojson)' "$R"
      edit '.spec.storage.nodes[0].devices[0].name="/dev/disk/by-id/wwn-a\tb"' "$C" ;;
  esac
  invoke
  "$check" "$label" "$expected" "$path"
  unset CASE_ENV STAT_KIND PROBE_STATUS MAP MINIMUM OSDS DRY_RUN EXPECT_DRY CONFLICT
}
scenarios=( "$F^^^-^^^$P^alias priority, equal size, retained FD through PATCH^capacity^"
"$F^$I=1048577^^-^^^$Z^below size^0^" "$F^$I=1048575^^-^^^$P^above size^capacity^" "$F^^^t^partition^^$Z^partition$E^0^"
 "$F^^^w^ro^1^$Z^readonly$E^0^" "$F^^^w^removable^1^$Z^removable$E^0^" "$F^STAT_KIND=regular^^-^^^$Z^type$E^0^"
 "$F^${S}=0^^-^^^$Z^signed$E^0^" "$F^^^u^^^$Z^unavailable$E^0^" "$F^MAP={}^^-^^^$Z^unmapped$E^0^"
 "$F^${S}=254^^-^^^$Z^ambivalent skip^0^" "$F^${S}=255^^-^^^$Z^probe error^1^" "$F^${S}=retarget^^-^^^$Z^$D^1^"
 "$F^^^w^dev^8:1^$Z^dev_t mismatch^1^" "$F^MAP=$SM^^s^^^$P^generic SSD class^fast^"
 "$V^MAP=$NM^^-^^^$P^direct NVMe class^direct^$NP" "$V^MAP=$M^^-^^^$P^direct NVMe media fallback^flash^$NP"
 "$V^MAP=$NM^^r^nsid^^$Z^${NS}missing$Q^1^" "$V^MAP=$NM^^w^nsid^bad^$Z^${NS}bad$Q^1^"
 "$V^MAP=$NM^^w^nsid^0^$Z^${NS}0$Q^1^" "$V^MAP=$NM^^r^device^^$P^non-PCI NVMe fallback^flash^$NP"
 "$F^^^q^^.[0].cephVolumeData=\"{\"^$Z^${RK}nested$Q^1^" "$F^^^q^^del(.[0].devLinks)^$Z^${RK}missing-links$Q^1^"
 "$F^^^q^^.[0].readOnly=true^$Z^${RK}readOnly$Q^1^" "$F^^^q^^.[0][\"kernel-name\"]=\"sdb\"^$Z^${RK}kernel$Q^1^"
 "$F^^^q^^.[0].devLinks=\"/dev/disk/by-id/../sda\"^$Z^${RK}bad-links$Q^1^" "$F^^^e^R^.items=[]^$Z^zero$Q^1^"
 "$F^^^e^R^.items += [.items[0]]^$Z^many$Q^1^" "$F^^^e^R^.items[0].data|=del(.devices)^$Z^absent$Q^1^"
 "$F^^^e^C^.spec.storage.nodes[0].devices|=.+[.[0]]^$Z^duplicate$Q^1^" "$F^^^x^^^$Z^cross$Q^1^"
"$F^^^e^C^.spec.storage.deviceFilter=\"sd.*\"^$Z^broad$Q^1^" "$F^^^e^N^.metadata.labels[\"$K\"]=\"Compute\"^$Z^$O$Q^1^"
 "$F^MAP={\"hdd\":}^^-^^^$Z^policy$Q^1^" "$F^DRY_RUN=false^EXPECT_DRY=false^-^^^$P^live exact patch flags^capacity^"
 "$F^${S}=0^^g^^^$Z^registered signed preservation and no-op^0^" "$F^^^w^size^36028797018966016^$Z^sectors$Q^1^"
 "$F^${I}=18446744073709551616^^-^^^$Z^minimum$Q^1^" "$F^OSDS=$HN^^-^^^$Z^osds$Q^1^"
"$F^^^e^C^.spec.storage.nodes[0].devices[0].name=\"disk/by-id/wwn-new\"^$Z^$RL^1^" "$V^MAP=$NM^^w^nsid^$HN^$Z^nsid$Q^1^"
 "$F^^^l^^^$Z^tab$Q^1^" "$F^^^a^^^$Z^no devices^0^" "$F^CONFLICT=true^^-^^^conflict^409 stops replay^^"
 "true^CONFLICT=true^^e^C^.metadata.resourceVersion=\"8\"^$P^fresh process after 409^capacity^")
for row in "${scenarios[@]}"
do
  IFS=^ read -r device env1 env2 action argument value check label expected path <<<"$row"
  scenario
done
verify "$enabled" registry/openark:v1 '{}'
verify "$fallback" registry/openark:2.0.0-alpha.2 '{}'
verify "$custom" r:v '{"nvme":"fast"}'
pass 'renders and lifecycle'
printf 'All %d cases passed.\n' "$CASES"
