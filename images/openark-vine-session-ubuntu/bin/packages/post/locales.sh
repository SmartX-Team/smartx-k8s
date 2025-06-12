#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Set default locale

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

locale-gen --purge en_US.UTF-8
echo 'LANG="en_US.UTF-8"' >/etc/default/locale
echo 'LANGUAGE="en_US:en"' >>/etc/default/locale
