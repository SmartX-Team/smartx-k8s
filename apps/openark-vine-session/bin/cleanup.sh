#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail
# Verbose
set -x


# Cleanup VMs
{{- if .Values.vm.enabled }}
if ! kubectl delete pods \
        --grace-period=120 \
        --label-selector "app.kubernetes.io/name={{ include "helm.fullname" $ }},kubevirt.io=virt-launcher"; then
    kubectl delete pods \
        --force \
        --label-selector "app.kubernetes.io/name={{ include "helm.fullname" $ }},kubevirt.io=virt-launcher"
fi
kubectl delete virtualmachineinstances.kubevirt.io {{ include "helm.fullname" $ | quote }} || true
{{- if .Values.persistence.enabled }}
kubectl delete virtualmachines.kubevirt.io {{ include "helm.fullname" $ | quote }} || true
{{- end }}
{{- end }}

# Cleanup DataVolumes
{{- if .Values.vm.enabled }}
kubectl delete datavolumes.cdi.kubevirt.io "{{ include "helm.localPVCName" $ }}-vm-cdrom-{{ .Values.vm.os }}" || true
kubectl delete persistentvolumeclaims "{{ include "helm.localPVCName" $ }}-vm-cdrom-{{ .Values.vm.os }}" || true
kubectl delete persistentvolumeclaims "{{ include "helm.localPVCName" $ }}-vm-cdrom-{{ .Values.vm.os }}-scratch" || true
kubectl delete persistentvolumes "{{ include "helm.localPVName" $ }}-vm-cdrom-{{ .Values.vm.os }}" || true
kubectl delete persistentvolumes "{{ include "helm.localPVName" $ }}-vm-cdrom-{{ .Values.vm.os }}-scratch" || true
{{- end }}

# Cleanup PVs
{{- if .Values.vm.enabled }}
kubectl delete persistentvolumeclaims "{{ include "helm.localPVCName" $ }}-vm-shared-{{ .Values.vm.os }}" || true
kubectl delete persistentvolumes "{{ include "helm.localPVName" $ }}-vm-shared-{{ .Values.vm.os }}" || true
{{- else }}
kubectl delete persistentvolumeclaims {{ include "helm.localPVCName" $ | quote }} || true
kubectl delete persistentvolumes {{ include "helm.localPVName" $ | quote }} || true
{{- end }}

exec true
