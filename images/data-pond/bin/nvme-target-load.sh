#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Data Pond Storage
# Unload NVMe Kernel Modules

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Set NVMe NQN
echo "${NODE_ID}" >/etc/nvme/hostid
echo "nqn.2014-08.org.nvmexpress:uuid:${NODE_ID}" >/etc/nvme/hostnqn

# nvme
modprobe nvme num_p2p_queues=1
modprobe nvme-fabrics
modprobe nvme-tcp
