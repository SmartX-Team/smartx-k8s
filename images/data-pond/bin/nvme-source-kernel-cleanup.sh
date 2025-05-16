#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Cleanup NVMe kernel modules

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Cleanup Ports
for port in $(find /sys/kernel/config/nvmet/ports -maxdepth 1 -mindepth 1 -type d); do
    find "${port}/subsystems" -maxdepth 1 -mindepth 1 -type l -exec rm '{}' \;
    rmdir "${port}"
done

# Cleanup Subsystems
for sys in $(find /sys/kernel/config/nvmet/subsystems -maxdepth 1 -mindepth 1 -type d); do
    for ns in $(find "${sys}/namespaces" -maxdepth 1 -mindepth 1 -type d); do
        echo 0 >"${ns}/enable"
        rmdir "${ns}"
    done
    rmdir "${sys}"
done
