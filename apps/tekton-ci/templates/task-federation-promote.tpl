{{- if and .Values.ci.enabled .Values.promotion.enabled }}
---
apiVersion: tekton.dev/v1
kind: Task
metadata:
  name: {{ required "promotion.names.task is required" .Values.promotion.names.task | quote }}
  namespace: {{ .Release.Namespace | quote }}
  labels:
{{- include "tekton-ci.labels" . | nindent 4 }}
spec:
  description: Update one Federation release on a bot branch and open a human-reviewed pull request.
  params:
    - name: payload
      type: string
  results:
    - name: branch
      type: string
      description: Promotion branch pushed by the bot.
    - name: changed
      type: string
      description: Whether the payload produced a Federation diff.
    - name: pull-request-url
      type: string
      description: Pull request URL, empty when no change was required.
  stepTemplate:
    securityContext:
      runAsNonRoot: true
      runAsUser: 65532
      runAsGroup: 65532
      allowPrivilegeEscalation: false
      capabilities:
        drop:
          - ALL
      seccompProfile:
        type: RuntimeDefault
    volumeMounts:
      - name: work
        mountPath: /workspace
  volumes:
    - name: work
      emptyDir: {}
  steps:
    - name: clone
      image: {{ required "ci.images.git is required" .Values.ci.images.git | quote }}
      computeResources: &promotionStepResources
        requests:
          cpu: 10m
          memory: 32Mi
        limits:
          cpu: 500m
          memory: 512Mi
      env:
        - name: FEDERATION_REPO_URL
          value: {{ required "promotion.federation.repoURL is required" .Values.promotion.federation.repoURL | quote }}
        - name: FEDERATION_BASE_BRANCH
          value: {{ required "promotion.federation.baseBranch is required" .Values.promotion.federation.baseBranch | quote }}
      script: |
        #!/bin/sh
        set -eu
        git clone --branch "$FEDERATION_BASE_BRANCH" --single-branch "$FEDERATION_REPO_URL" /workspace/repository

    - name: update-release
      image: {{ required "promotion.images.yq is required" .Values.promotion.images.yq | quote }}
      computeResources: *promotionStepResources
      env:
        - name: PAYLOAD
          value: $(params.payload)
      script: |
        #!/bin/sh
        set -eu

        printf '%s' "$PAYLOAD" >/workspace/payload.json
        schema="$(yq -r '.schemaVersion // ""' /workspace/payload.json)"
        child="$(yq -r '.childName // ""' /workspace/payload.json)"
        source_revision="$(yq -r '.sourceRevision // ""' /workspace/payload.json)"
        pipeline_run="$(yq -r '.pipelineRun // ""' /workspace/payload.json)"


        [ "$schema" = v1 ] || { printf 'unsupported promotion payload schema\n' >&2; exit 1; }
        printf '%s' "$child" | grep -Eq '^[a-z0-9]([-a-z0-9]*[a-z0-9])?$' || {
          printf 'invalid child name\n' >&2; exit 1;
        }
        [ "${#source_revision}" -eq 40 ] || { printf 'invalid source revision\n' >&2; exit 1; }
        case "$source_revision" in *[!0-9a-f]*) printf 'invalid source revision\n' >&2; exit 1;; esac
        SOURCE_REVISION="$source_revision" yq -e '
          (.images | length) > 0 and
          ([.images[] | (
            .sourceRevision == strenv(SOURCE_REVISION) and
            .immutableTag == ("sha-" + strenv(SOURCE_REVISION)) and
            (.tag | test("^v(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)(-[0-9A-Za-z]+([.-][0-9A-Za-z]+)*)?$")) and
            (.digest | test("^sha256:[0-9a-f]{64}$"))
          )] | all)
        ' /workspace/payload.json >/dev/null || { printf 'invalid atomic image set\n' >&2; exit 1; }

        release_file="/workspace/repository/releases/${child}/release.yaml"
        values_file="/workspace/repository/releases/${child}/values.yaml"
        [ -f "$release_file" ] || { printf 'release not enrolled: %s\n' "$child" >&2; exit 1; }
        [ -s "$values_file" ] || printf '{}\n' >"$values_file"

        release_name="$(yq -r '.name // ""' "$release_file")"
        [ "$release_name" = "$child" ] || { printf 'release identity mismatch\n' >&2; exit 1; }
        # The payload defines the image set. Adding or removing an image no longer
        # requires a preparatory human edit of the Federation values, and a first
        # enrollment no longer needs a hand-seeded placeholder digest.
        # Atomicity is unchanged: values.yaml is rewritten in full, never patched.
        # The authoritative cross-check is the child chart inventory, verified by
        # the verify-image-inventory step once the child source is checked out.
        existing_keys="$(yq -r '(.images // {}) | keys | sort | join(",")' "$values_file")"
        payload_keys="$(yq -r '[.images[].name] | sort | join(",")' /workspace/payload.json)"
        printf '%s' "$payload_keys" >/workspace/payload-image-keys
        if [ -z "$existing_keys" ]; then
          printf 'initial image set: %s' "$payload_keys" >/workspace/image-set-change
        elif [ "$existing_keys" = "$payload_keys" ]; then
          printf 'image set unchanged: %s' "$payload_keys" >/workspace/image-set-change
        else
          printf 'image set changed: %s -> %s' "$existing_keys" "$payload_keys" >/workspace/image-set-change
        fi
        explicit_revision="$(yq -r '.source.revision // ""' "$release_file")"
        mode="$(yq -r '.promotion.mode // ""' "$release_file")"
        yq -r '.promotion.resolvedRevision // ""' "$release_file" >/workspace/previous-resolved-revision
        if [ -n "$explicit_revision" ]; then
          [ "$mode" = pinned ] || { printf 'explicit revision requires pinned mode\n' >&2; exit 1; }
          [ "$explicit_revision" = "$source_revision" ] || {
            printf 'release is pinned to %s; refusing candidate %s\n' "$explicit_revision" "$source_revision" >&2
            exit 1
          }
        else
          [ "$mode" = tracking ] || { printf 'revision-less release requires tracking mode\n' >&2; exit 1; }
          SOURCE_REVISION="$source_revision" yq -i \
            '.promotion.resolvedRevision = strenv(SOURCE_REVISION)' "$release_file"
        fi

        yq -r '.images[] | [.name, .repository, .tag, .digest, .sourceRevision] | @tsv' \
          /workspace/payload.json >/workspace/images.tsv
        printf 'images: {}\n' >"$values_file"
        tab="$(printf '\t')"
        while IFS="$tab" read -r image_key image_repository image_tag image_digest image_revision; do
          IMAGE_KEY="$image_key" IMAGE_REPOSITORY="$image_repository" IMAGE_TAG="$image_tag" \
          IMAGE_DIGEST="$image_digest" SOURCE_REVISION="$image_revision" yq -i '
            .images[strenv(IMAGE_KEY)].repository = strenv(IMAGE_REPOSITORY) |
            .images[strenv(IMAGE_KEY)].tag = strenv(IMAGE_TAG) |
            .images[strenv(IMAGE_KEY)].pullPolicy = "IfNotPresent" |
            .images[strenv(IMAGE_KEY)].digest = strenv(IMAGE_DIGEST) |
            .images[strenv(IMAGE_KEY)].sourceRevision = strenv(SOURCE_REVISION)
          ' "$values_file"
        done </workspace/images.tsv

        yq -r '.source.repoURL' "$release_file" >/workspace/source-repo
        yq -r '.source.branch // "main"' "$release_file" >/workspace/source-branch
        yq -r '.source.path' "$release_file" >/workspace/source-path
        printf '%s' "$child" >/workspace/child
        printf '%s' "$source_revision" >/workspace/source-revision
        printf '%s' "$pipeline_run" >/workspace/pipeline-run

    - name: verify-candidate
      image: {{ required "ci.images.git is required" .Values.ci.images.git | quote }}
      computeResources: *promotionStepResources
      script: |
        #!/bin/sh
        set -eu
        source_repo="$(cat /workspace/source-repo)"
        source_branch="$(cat /workspace/source-branch)"
        candidate="$(cat /workspace/source-revision)"
        previous="$(cat /workspace/previous-resolved-revision)"
        git init -b main /workspace/child-source
        git -C /workspace/child-source remote add origin "$source_repo"
        git -C /workspace/child-source fetch origin "refs/heads/${source_branch}:refs/remotes/origin/${source_branch}"
        head_revision="$(git -C /workspace/child-source rev-parse "origin/${source_branch}")"
        git -C /workspace/child-source merge-base --is-ancestor "$candidate" "origin/${source_branch}" || {
          printf 'candidate is not reachable from %s\n' "$source_branch" >&2; exit 1;
        }
        [ "$candidate" = "$head_revision" ] || {
          printf 'stale build: branch head is %s, candidate is %s\n' "$head_revision" "$candidate" >&2; exit 1;
        }
        if [ -n "$previous" ] && [ "$previous" != "$candidate" ]; then
          git -C /workspace/child-source cat-file -e "${previous}^{commit}" 2>/dev/null || \
            git -C /workspace/child-source fetch origin "$previous"
          git -C /workspace/child-source merge-base --is-ancestor "$previous" "$candidate" || {
            printf 'candidate diverges from previously promoted revision\n' >&2; exit 1;
          }
        fi
        git -C /workspace/child-source checkout --detach "$candidate"

    - name: verify-image-inventory
      image: {{ .Values.promotion.images.yq | quote }}
      computeResources: *promotionStepResources
      script: |
        #!/bin/sh
        set -eu
        # The child chart values.yaml image map is the authoritative inventory.
        # Comparing the payload against it - rather than against the previously
        # promoted Federation values - allows the set to change while still
        # rejecting a partial build that would leave some workloads without a
        # digest.
        chart_values="/workspace/child-source/$(cat /workspace/source-path)/values.yaml"
        [ -f "$chart_values" ] || {
          printf 'child chart values not found: %s\n' "$chart_values" >&2; exit 1;
        }
        chart_keys="$(yq -r '(.images // {}) | keys | sort | join(",")' "$chart_values")"
        payload_keys="$(cat /workspace/payload-image-keys)"
        [ -n "$chart_keys" ] || {
          printf 'child chart declares no images but the payload promotes %s\n' "$payload_keys" >&2
          exit 1
        }
        [ "$chart_keys" = "$payload_keys" ] || {
          printf 'payload does not match the child chart image inventory: chart=%s payload=%s\n' \
            "$chart_keys" "$payload_keys" >&2
          exit 1
        }

    - name: render-candidate
      image: {{ required "ci.images.helm is required" .Values.ci.images.helm | quote }}
      computeResources: *promotionStepResources
      script: |
        #!/bin/sh
        set -eu
        child="$(cat /workspace/child)"
        chart="/workspace/child-source/$(cat /workspace/source-path)"
        runtime="/workspace/repository/releases/${child}/runtime-values.yaml"
        generated="/workspace/repository/releases/${child}/values.yaml"
        [ -f "$runtime" ] && [ -f "$generated" ] || { printf 'promotion values missing\n' >&2; exit 1; }
        helm lint "$chart" -f "$runtime" -f "$generated"
        helm template "$child" "$chart" -f "$runtime" -f "$generated" >/workspace/candidate-rendered.yaml
        [ -s /workspace/candidate-rendered.yaml ] || { printf 'candidate Helm render is empty\n' >&2; exit 1; }

    - name: mint-app-jwt
      image: {{ required "promotion.images.openssl is required" .Values.promotion.images.openssl | quote }}
      computeResources: *promotionStepResources
      env:
        - name: GITHUB_APP_ID
          valueFrom:
            secretKeyRef:
              name: {{ required "promotion.github.credentialsSecretName is required" .Values.promotion.github.credentialsSecretName | quote }}
              key: {{ required "promotion.github.appIDKey is required" .Values.promotion.github.appIDKey | quote }}
        - name: GITHUB_APP_PRIVATE_KEY
          valueFrom:
            secretKeyRef:
              name: {{ .Values.promotion.github.credentialsSecretName | quote }}
              key: {{ required "promotion.github.privateKeyKey is required" .Values.promotion.github.privateKeyKey | quote }}
      script: |
        #!/bin/sh
        set -eu

        case "$GITHUB_APP_ID" in
          ''|*[!0-9]*) printf 'GitHub App ID must be numeric\n' >&2; exit 1 ;;
        esac
        b64url() {
          openssl base64 -A | tr '+/' '-_' | tr -d '='
        }
        now="$(date +%s)"
        issued_at="$((now - 60))"
        expires_at="$((now + 540))"
        umask 077
        printf '%s' "$GITHUB_APP_PRIVATE_KEY" >/workspace/github-app-private-key.pem
        unset GITHUB_APP_PRIVATE_KEY
        header="$(printf '%s' '{"alg":"RS256","typ":"JWT"}' | b64url)"
        claims="$(printf '{"iat":%s,"exp":%s,"iss":"%s"}' "$issued_at" "$expires_at" "$GITHUB_APP_ID" | b64url)"
        unsigned="${header}.${claims}"
        signature="$(printf '%s' "$unsigned" | openssl dgst -sha256 -sign /workspace/github-app-private-key.pem -binary | b64url)"
        printf '%s.%s' "$unsigned" "$signature" >/workspace/github-app-jwt
        rm -f /workspace/github-app-private-key.pem

    - name: request-installation-token
      image: {{ required "promotion.images.curl is required" .Values.promotion.images.curl | quote }}
      computeResources: *promotionStepResources
      env:
        - name: GITHUB_INSTALLATION_ID
          valueFrom:
            secretKeyRef:
              name: {{ .Values.promotion.github.credentialsSecretName | quote }}
              key: {{ required "promotion.github.installationIDKey is required" .Values.promotion.github.installationIDKey | quote }}
      script: |
        #!/bin/sh
        set -eu
        case "$GITHUB_INSTALLATION_ID" in
          ''|*[!0-9]*) printf 'GitHub installation ID must be numeric\n' >&2; exit 1 ;;
        esac
        jwt="$(cat /workspace/github-app-jwt)"
        auth_header="Authorization: Bearer ${jwt}"
        status="$(curl -sS -o /workspace/installation-token.json -w '%{http_code}' \
          -X POST \
          -H 'Accept: application/vnd.github+json' \
          -H "$auth_header" \
          -H 'X-GitHub-Api-Version: 2022-11-28' \
          "https://api.github.com/app/installations/${GITHUB_INSTALLATION_ID}/access_tokens")"
        [ "$status" = 201 ] || {
          printf 'GitHub installation-token API returned HTTP %s\n' "$status" >&2
          sed -n '1,80p' /workspace/installation-token.json >&2
          exit 1
        }
        rm -f /workspace/github-app-jwt

    - name: extract-installation-token
      image: {{ .Values.promotion.images.yq | quote }}
      computeResources: *promotionStepResources
      script: |
        #!/bin/sh
        set -eu
        token="$(yq -r '.token // ""' /workspace/installation-token.json)"
        [ -n "$token" ] || { printf 'GitHub installation token missing\n' >&2; exit 1; }
        umask 077
        printf '%s' "$token" >/workspace/github-token
        rm -f /workspace/installation-token.json

    - name: commit-and-push
      image: {{ required "ci.images.git is required" .Values.ci.images.git | quote }}
      computeResources: *promotionStepResources
      script: |
        #!/bin/sh
        set -eu
        GITHUB_TOKEN="$(cat /workspace/github-token)"
        export GITHUB_TOKEN
        cd /workspace/repository

        child="$(cat /workspace/child)"
        source_revision="$(cat /workspace/source-revision)"

        allowed_release="releases/${child}/release.yaml"
        allowed_values="releases/${child}/values.yaml"

        # Untracked files must be inspected too: on a first enrollment
        # releases/<child>/values.yaml does not exist yet and is created by the
        # update-release step, and `git diff` cannot see it. Using `git diff`
        # alone would silently report "no change" for a pinned first enrollment
        # and never open a pull request.
        changed_paths="$(git status --porcelain --untracked-files=all | cut -c4-)"
        for changed in $changed_paths; do
          [ "$changed" = "$allowed_release" ] || [ "$changed" = "$allowed_values" ] || {
            printf 'promotion modified forbidden path: %s\n' "$changed" >&2
            exit 1
          }
        done

        short_revision="$(printf '%s' "$source_revision" | cut -c1-12)"
        branch="ci/promote-${child}"
        printf '%s' "$branch" >"$(results.branch.path)"

        if [ -z "$changed_paths" ]; then
          printf 'false' >"$(results.changed.path)"
          : >"$(results.pull-request-url.path)"
          touch /workspace/no-change
          exit 0
        fi

        git config user.name 'ScaleX Federation Promotion'
        git config user.email 'scalex-federation-bot@users.noreply.github.com'
        git config credential.helper '!f() { echo username=x-access-token; echo password="$GITHUB_TOKEN"; }; f'
        remote_revision="$(git ls-remote --heads origin "refs/heads/${branch}" | cut -f1)"
        git checkout -B "$branch"
        git add "$allowed_release" "$allowed_values"
        git commit -s -m "chore(${child}): promote ${short_revision}"
        if [ -n "$remote_revision" ]; then
          git push --force-with-lease="refs/heads/${branch}:${remote_revision}" \
            --set-upstream origin "HEAD:refs/heads/${branch}"
        else
          git push --force-with-lease="refs/heads/${branch}:" \
            --set-upstream origin "HEAD:refs/heads/${branch}"
        fi
        printf 'true' >"$(results.changed.path)"

    - name: open-pull-request
      image: {{ required "promotion.images.curl is required" .Values.promotion.images.curl | quote }}
      computeResources: *promotionStepResources
      env:
        - name: GITHUB_REPOSITORY
          value: {{ required "promotion.federation.githubRepository is required" .Values.promotion.federation.githubRepository | quote }}
        - name: FEDERATION_BASE_BRANCH
          value: {{ .Values.promotion.federation.baseBranch | quote }}
      script: |
        #!/bin/sh
        set -eu
        GITHUB_TOKEN="$(cat /workspace/github-token)"
        if [ -f /workspace/no-change ]; then
          printf '{"html_url":""}\n' >/workspace/pull-request.json
          exit 0
        fi

        branch="$(cat "$(results.branch.path)")"
        child="$(cat /workspace/child)"
        source_revision="$(cat /workspace/source-revision)"
        head_owner="${GITHUB_REPOSITORY%%/*}"
        auth_header="Authorization: Bearer ${GITHUB_TOKEN}"
        curl -fsS \
          -H 'Accept: application/vnd.github+json' \
          -H "$auth_header" \
          -H 'X-GitHub-Api-Version: 2022-11-28' \
          "https://api.github.com/repos/${GITHUB_REPOSITORY}/pulls?state=open&base=${FEDERATION_BASE_BRANCH}&head=${head_owner}:${branch}" \
          >/workspace/existing-pull-requests.json
        if grep -q '"html_url"' /workspace/existing-pull-requests.json; then
          mv /workspace/existing-pull-requests.json /workspace/pull-request.json
          exit 0
        fi
        rm -f /workspace/existing-pull-requests.json
        title="chore(${child}): promote ${source_revision}"
        image_set_change="$(cat /workspace/image-set-change 2>/dev/null || printf 'unknown')"
        body="Automated promotion from Tekton. Human review and merge are required.\\n\\n${image_set_change}"
        request="$(printf '{"title":"%s","head":"%s","base":"%s","body":"%s"}' \
          "$title" "$branch" "$FEDERATION_BASE_BRANCH" "$body")"
        status="$(curl -sS -o /workspace/pull-request.json -w '%{http_code}' \
          -X POST \
          -H 'Accept: application/vnd.github+json' \
          -H "$auth_header" \
          -H 'X-GitHub-Api-Version: 2022-11-28' \
          "https://api.github.com/repos/${GITHUB_REPOSITORY}/pulls" \
          -d "$request")"
        [ "$status" = 201 ] || {
          printf 'GitHub pull request API returned HTTP %s\n' "$status" >&2
          sed -n '1,80p' /workspace/pull-request.json >&2
          exit 1
        }

    - name: emit-result
      image: {{ .Values.promotion.images.yq | quote }}
      computeResources: *promotionStepResources
      script: |
        #!/bin/sh
        set -eu
        url="$(yq -r '[.] | flatten | .[0].html_url // ""' /workspace/pull-request.json)"
        printf '%s' "$url" >"$(results.pull-request-url.path)"
        rm -f /workspace/github-token /workspace/pull-request.json
{{- end }}
