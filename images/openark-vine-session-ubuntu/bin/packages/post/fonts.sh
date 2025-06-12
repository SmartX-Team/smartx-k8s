#!/usr/bin/env bash
# Copyright (c) 2023 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Link default system-wide font config

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

ln -sf {{ printf "/home/%s/.config/fontconfig/conf.d/99-openark.conf" .Values.user.name | quote }} /etc/fonts/conf.d/99-openark.conf
rm -f /etc/fonts/conf.d/65-nonlatin.conf
