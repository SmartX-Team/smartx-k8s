#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Prehibit errors
set -e -o pipefail

###########################################################
#   Configuration                                         #
###########################################################

# Can be one of: github
GIT_REMOTE_KIND="github"

###########################################################
#   Execute CI                                            #
###########################################################

exec "$(dirname "$0")/${GIT_REMOTE_KIND}-pr-check.sh"
