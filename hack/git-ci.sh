#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

###########################################################
#   Configuration                                         #
###########################################################

# Can be one of: github
GIT_REMOTE_KIND="github"

###########################################################
#   Validate Commit(s)                                    #
###########################################################

"$(dirname "$0")/git-ci-validate.sh"

export CHECK_STATUS='completed'
if [ $? == '0' ]; then
    export CHECK_CONCLUSION='success'
    export GIT_CI_SUMMARY='Succeeded'
    export GIT_CI_TEXT='# Succeeded'
else
    export CHECK_CONCLUSION='failure'
    export GIT_CI_SUMMARY='Failed'
    export GIT_CI_TEXT='# Failed'
fi

###########################################################
#   Execute CI                                            #
###########################################################

# Prehibit errors
set -e -o pipefail

"$(dirname "$0")/${GIT_REMOTE_KIND}-pr-check.sh"
