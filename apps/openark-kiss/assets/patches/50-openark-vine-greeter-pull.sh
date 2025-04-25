#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Greeting Screen Configuration
# Pull an image

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

{{- if has "org.ulagbulag.io/desktop-environment/vine" .Values.features }}

# Start ContainerD
containerd &
containerd_pid="$!"

# Wait for ContainerD to be ready
until [ -S /run/containerd/containerd.sock ]; do
    sleep 1
done

nerdctl pull --quiet "{{ .Values.greeter.image.repo }}:{{ .Values.greeter.image.tag | default .Chart.AppVersion }}"

# Stop ContainerD
kill "${containerd_pid}" || true
wait "${containerd_pid}" || true

{{- end }}
