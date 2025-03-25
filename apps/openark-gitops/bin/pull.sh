#!/usr/bin/env bash
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# OpenARK GitOps - Pull a repository

# Prehibit errors
set -e -o pipefail
# Verbose
set -x

# Configure Git
git config --global init.defaultBranch "{{ workflow.parameters.commit_branch }}"
git config --global --add safe.directory "/home/user{{ workflow.parameters.workdir }}"

# Init a workdir
mkdir -p "/home/user{{ workflow.parameters.workdir }}"
cd "/home/user{{ workflow.parameters.workdir }}"

# Pull a repository
git init
git remote add origin "{{ workflow.parameters.base_url }}/{{ workflow.parameters.repo_owner }}/{{ workflow.parameters.repo_name }}"
git pull origin "{{ workflow.parameters.commit_branch }}"
git switch "{{ workflow.parameters.commit_branch }}"
git branch --set-upstream-to "origin/{{ workflow.parameters.commit_branch }}" "{{ workflow.parameters.commit_branch }}"
