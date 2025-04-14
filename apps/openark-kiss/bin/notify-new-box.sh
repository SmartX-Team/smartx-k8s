#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

# Collect node info
ADDRESS="$(ip route get 1.1.1.1 | grep -oP 'src \K\d+(\.\d+){3}' | head -1)"
UUID="$(cat /sys/class/dmi/id/product_uuid)"

# Submit to KISS Cluster
exec curl --retry 5 --retry-delay 5 \
    "http://gateway.{{ .Release.Namespace }}.svc.{{ include "helm.clusterDomainName" $ }}/new?address=${ADDRESS}&uuid=${UUID}"
