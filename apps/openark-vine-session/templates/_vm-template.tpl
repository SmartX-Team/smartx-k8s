{{/*
KubeVirt Virtual Machine GPU Passthrough devices
*/}}
{{- define "helm.vmGPUPassthroughDevices" -}}
{{- $_ := set $ "Counter" 0 }}
{{- range $key, $value := $.Values.session.resources.limits }}
{{- if regexMatch "^nvidia.com/[a-zA-Z0-9_]+$" $key }}
{{- range $_, $_ := until ( $value | int ) }}
{{- if $.Values.features.hostDisplay }}
- deviceName: {{ $key | quote }}
  name: "gpu-{{ $.Counter }}"
{{- end }}
{{- $_ := set $ "Counter" ( add $.Counter 1 ) }}
{{- if $.Values.features.hostAudio }}
- deviceName: "{{ $key }}_Audio"
  name: "gpu-{{ $.Counter }}"
{{- $_ := set $ "Counter" ( add $.Counter 1 ) }}
{{- end }}
{{- end }}
{{- end }}
{{- end }}
{{- $_ := unset $ "Counter" }}
{{- end }}

{{/*
KubeVirt Virtual Machine Instance metadata
*/}}
{{- define "helm.vmiMetadata" -}}
{{ include "helm.podMetadata" $ }}
{{- end }}

{{/*
KubeVirt Virtual Machine Instance template
*/}}
{{- define "helm.vmiTemplate" -}}

{{- if not .Values.features.desktopEnvironment }}
{{- fail "VM cannot be enabled without desktop environment" }}
{{- end }}

{{- if not .Values.features.vm }}
{{- fail ( printf "%s (%s)"
  "VM feature is not enabled"
  "You may want to enable \"org.ulagbulag.io/vm/kubevirt\" feature in the cluster"
) }}
{{- end }}

{{- if .Values.session.context.hostIPC }}
{{- fail "host IPC is not supported in VM" }}
{{- end }}

affinity:
{{- include "helm.affinity" $ | nindent 2 }}
domain:
  clock:
    timer:
      hpet:
        present: false
{{- if regexMatch "^windows-[0-9]+$" .Values.vm.os }}
      hyperv: {}
{{- end }}
      pit:
        tickPolicy: delay
      rtc:
        tickPolicy: catchup
    utc: {}
  cpu:
    dedicatedCpuPlacement: true
    isolateEmulatorThread: true
    # this passthrough the node CPU to the VM
    model: host-passthrough
{{- if eq 0 ( mod ( .Values.session.resources.limits.cpu | int ) 2 ) }}
    cores: {{ div ( .Values.session.resources.limits.cpu | int ) 2 }}
    threads: 2
{{- else if gt ( .Values.session.resources.limits.cpu | int ) 1 }}
    cores: {{ ( div ( .Values.session.resources.limits.cpu | int ) 2 ) | int }}
    threads: 2
{{- else }}
    cores: {{ .Values.session.resources.limits.cpu | int }}
    threads: 2
{{- end }}
    sockets: 1
    # features:
    #   - name: tsc-deadline
    #     policy: require
    # numa:
    #   guestMappingPassthrough: {}
    # realtime:
    #   mask: "0"
  devices:
{{- if .Values.features.hostDisplay }}
    autoattachGraphicsDevice: false
{{- else if not .Values.services.x11vnc.enabled }}
{{- fail "Virtual graphics device cannot be attached without x11vnc service in VM" }}
{{- else }}
    autoattachGraphicsDevice: true
{{- end }}
    # autoattachInputDevice: false
    autoattachMemBalloon: false
    autoattachSerialConsole: false
    autoattachVSOCK: false
{{- if not .Values.features.hostDisplay }}
    clientPassthrough: {} # allow remote device mounting
{{- end }}
    disks:
      - name: disk-a
        bootOrder: 1
        disk:
          bus: sata # FIXME: virtio, scsi not working
      - name: cdrom-iso
        bootOrder: 2
        cdrom: # FIXED
          bus: scsi # FIXED
{{- if regexMatch "^windows-[0-9]+$" .Values.vm.os }}
      - name: cdrom-virtio
        cdrom: # FIXED
          bus: sata # FIXED
      - name: sysprep
        cdrom: # FIXED
          bus: sata # FIXED
{{- end }}
    hostDevices:
{{- include "helm.vmGPUPassthroughDevices" $ | trim | nindent 6 }}
{{- if .Values.features.hostDisplay }}
{{- if not .Values.features.devicePassthrough }}
{{- fail "host display cannot be enabled without device passthrough in VM" }}
{{- end }}

{{- $_ := set $ "Counter" 0 }}
{{- range $_ := .Values.vm.hostDevices }}
      - deviceName: "{{ .apiGroup }}-{{ snakecase .kind }}-{{ .vendor }}-{{ .product }}"
        name: "host-{{ $.Counter }}"
{{- $_ := set $ "Counter" ( add $.Counter 1 ) }}
{{- end }}
{{- $_ := unset $ "Counter" }}

{{- end }}
    interfaces:
      - name: default
        model: virtio
        macAddress: de:ad:00:00:be:af # <--- this
        masquerade: {}
        ports:

{{- /********************************/}}
{{- if .Values.services.novnc.enabled }}
# TODO: to be implemented (sidecar container)
{{- fail "novnc service is not supported yet in VM" }}
{{- end }}

{{- /********************************/}}
{{- if .Values.services.rdp.enabled }}
{{- if not ( regexMatch "^windows-[0-9]+$" .Values.vm.os ) }}
{{- fail "RDP service is not supported in non-windows OS" }}
{{- end }}
          - name: rdp-tcp
            protocol: TCP
            port: 3389
          - name: rdp-udp
            protocol: UDP
            port: 3389
{{- end }}

{{- /********************************/}}
{{- if .Values.services.x11vnc.enabled }}
{{- if .Values.features.hostDisplay }}
{{- fail "x11vnc service is not supported with host display in VM" }}
{{- end }}
          - name: x11vnc
            protocol: TCP
            port: 5900
{{- end }}

    rng: {}
  features:
    acpi: {}
    apic: {}
{{- if regexMatch "^windows-[0-9]+$" .Values.vm.os }}
    hyperv:
      relaxed: {}
      spinlocks:
        spinlocks: 8191
      vapic: {}
    # hypervPassthrough:
    #   enabled: true
{{- end }}
    smm: {}
  firmware:
    bootloader:
      efi: {}
    # uuid: 5d307ca9-b3ef-428c-8861-06e72d69f223
    # serial: e4686d2c-6e8d-4335-b8fd-81bee22f4815
  ioThreadsPolicy: auto
  machine:
    type: q35
  # memory:
  #   hugepages:
  #     pageSize: 1Gi
  resources:
    limits:
{{- range $key, $value := .Values.session.resources.limits | default dict }}
{{- if not ( or
  ( eq "cpu" $key )
  ( regexMatch "^nvidia.com/[a-zA-Z0-9_]+$" $key )
) }}
      {{ $key | quote }}: {{ $value | quote }}
{{- end }}
{{- end }}
{{- if not ( empty ( .Values.session.resources.requests | default dict ) ) }}
    requests:
{{- .Values.session.resources.requests | toYaml | nindent 6 }}
{{- end }}
hostname: "{{ include "helm.fullname" $ }}-{{ .Release.Namespace }}"
networks:
{{- if .Values.session.context.hostNetwork }}
{{- fail "host network is not supported in VM" }}
{{- end }}
  - name: default
    pod: {}
priorityClassName: {{ .Values.session.priorityClassName | quote }}
terminationGracePeriodSeconds: 30
volumes:
{{- /********************************/}}
  - name: cdrom-iso
    dataVolume:
      name: "{{ include "helm.localPVCName" $ }}-vm-cdrom-{{ .Values.vm.os }}"

{{- /********************************/}}
{{- if regexMatch "^windows-[0-9]+$" .Values.vm.os }}
  - name: cdrom-virtio
    containerDisk:
      image: "{{ .Values.vm.windows.virtioContainerDisk.image.repo }}:{{ .Values.vm.windows.virtioContainerDisk.image.tag }}"
      imagePullPolicy: {{ .Values.vm.windows.virtioContainerDisk.image.pullPolicy | quote }}
{{- end }}

{{- /********************************/}}
  - name: disk-a
{{- if eq .Values.volumes.vm.type "LocalShared" }}
    persistentVolumeClaim:
      claimName: "{{ include "helm.localPVCName" $ }}-vm-shared-{{ .Values.vm.os }}"
      # hotpluggable: false
{{- else if eq .Values.volumes.vm.type "RemoteOwned" }}
    persistentVolumeClaim:
      claimName: "{{ include "helm.remotePVCName" $ }}-vm-{{ .Values.vm.os }}"
      # hotpluggable: false
{{- else }}
{{- fail ( printf "Unknown VM volume type: %s" .Values.volumes.vm.type ) }}
{{- end }}

{{- /********************************/}}
{{- if regexMatch "^windows-[0-9]+$" .Values.vm.os }}
  - name: sysprep
    sysprep:
      configMap:
        name: {{ include "helm.vm.sysprepName" $ | quote }}
{{- end }}

{{- end }}
