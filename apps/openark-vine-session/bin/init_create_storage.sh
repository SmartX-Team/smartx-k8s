#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

{{- /* Collect volumes */}}
{{- $_ := set $ "Volumes" ( dict
    "root" list
    "user" list
) }}
{{- if .Values.vm.enabled }}

{{- /********************************/}}
{{- $_ := set $.Volumes "user" ( append $.Volumes.user (
    include "helm.localPVPath.vm.cdrom" $
) ) }}
{{- $_ := set $.Volumes "user" ( append $.Volumes.user (
    include "helm.localPVPath.vm.cdrom.scratch" $
) ) }}

{{- /********************************/}}
{{- if eq .Values.volumes.vm.type "LocalShared" }}
{{- $_ := set $.Volumes "user" ( append $.Volumes.user (
    include "helm.localPVPath.vm.shared" $
) ) }}
{{- end }}

{{- else }}

{{- /********************************/}}
{{- if and
    .Values.features.containers ( or
    ( eq .Values.volumes.home.type "LocalOwned" )
    ( eq .Values.volumes.home.type "LocalShared" )
) }}
{{- $containerHome := printf "%s/%s"
    ( include "helm.localPVPath" $ )
    ( include "helm.userContainersHomeSubPath" $ )
}}
{{- if or .Values.session.context.root .Values.volumes.container.root }}
{{- $_ := set $.Volumes "root" ( append $.Volumes.root $containerHome ) }}
{{- $_ := set $.Volumes "root" ( append $.Volumes.root ( printf "%s/storage"
   $containerHome
) ) }}
{{- else }}
{{- $_ := set $.Volumes "user" ( append $.Volumes.user $containerHome ) }}
{{- $_ := set $.Volumes "user" ( append $.Volumes.user ( printf "%s/storage"
   $containerHome
) ) }}
{{- end }}
{{- end }}

{{- /********************************/}}
{{- if and
    .Values.features.data ( or
    ( eq .Values.volumes.home.type "LocalOwned" )
    ( eq .Values.volumes.home.type "LocalShared" )
) }}
{{- $_ := set $.Volumes "user" ( append $.Volumes.user ( printf "%s/%s"
    ( include "helm.localPVPath" $ ) (
        include "helm.userDataHomeSubPath" $
    )
) ) }}
{{- end }}

{{- /********************************/}}
{{- if or
  ( eq .Values.volumes.home.type "LocalOwned" )
  ( eq .Values.volumes.home.type "LocalShared" )
}}
{{- $_ := set $.Volumes "user" ( append $.Volumes.user ( printf "%s/%s"
    ( include "helm.localPVPath" $ ) (
        include "helm.userHomeSubPath" $
    )
) ) }}

{{- /********************************/}}
{{- if .Values.services.ssh.enabled }}
{{- $_ := set $.Volumes "user" ( append $.Volumes.user ( printf "%s/%s"
    ( include "helm.localPVPath" $ ) (
        include "helm.userSshHomeSubPath" $
    )
) ) }}
{{- end }}
{{- end }}

{{- end }}

# Create local volumes
{{- range $_ := $.Volumes.root }}
mkdir -p {{ . | quote }}
chown -R "0:0" {{ . | quote }}
{{- end }}
{{- range $_ := $.Volumes.user }}
mkdir -p {{ . | quote }}
chown -R "${TARGET_UID}:${TARGET_GID}" {{ . | quote }}
{{- end }}

exec true
