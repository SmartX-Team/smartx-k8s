# Federation promotion 수동 검증

## 전제조건

- `Pipeline/child-build`가 `Succeeded=True`이고 `promotion-payload` result가 존재한다.
- `tower-ci/Secret/federation-promotion-github-app`이 존재하며 키 이름은
  `appID`, `installationID`, `privateKey`다.
- GitHub App installation은 `SJoon99/scalex-federation`의 Contents 및 Pull requests
  read/write 권한을 가진다.

Secret 값은 Git에 저장하지 않는다.

Promotion Task는 private key로 10분 이내 유효한 App JWT를 만들고, 매 실행마다 짧은
수명의 installation token을 발급한다. static PAT은 사용하지 않는다.

## PromotionRun 생성

성공한 build PipelineRun 이름을 지정한다.

```bash
BUILD_RUN=<successful-child-build-pipelinerun>
PAYLOAD="$(tkubectl -n tower-ci get pipelinerun "$BUILD_RUN" -o json | \
  jq -r '.status.results[] | select(.name == "promotion-payload") | .value.stringVal // .value')"
```

템플릿을 복사한 뒤 `__PROMOTION_PAYLOAD_JSON__`을 payload JSON으로 치환해 실행한다. JSON은 YAML single-quoted scalar에 들어가므로 payload에 single quote를 허용하지 않는다.

```bash
cp federation-promotion-pipelinerun.yaml.tpl /tmp/federation-promotion-run.yaml
PAYLOAD="$PAYLOAD" yq -i '.spec.params[0].value = strenv(PAYLOAD)' /tmp/federation-promotion-run.yaml
tkubectl create -f /tmp/federation-promotion-run.yaml
```

확인:

```bash
tkubectl -n tower-ci get pipelinerun,taskrun,pod
tkubectl -n tower-ci logs -l tekton.dev/pipelineRun --all-containers --prefix
```

성공 조건:

```text
PipelineRun Succeeded=True
results.changed=true
results.branch=ci/promote-...
results.pull-request-url=https://github.com/SmartX-Team/sandbox-scalex-federation/pull/...
```

PR에서 허용되는 변경은 다음뿐이다.

```text
releases/<child>/release.yaml
releases/<child>/values.yaml
```

`main` merge는 사람이 수행한다. Promotion Pipeline은 Argo sync나 Karmada apply를 호출하지 않는다.
