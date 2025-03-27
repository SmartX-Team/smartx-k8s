# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

# Build and push an image within ./images
build-image NAME:
    clear
    IMAGE_NAME={{ NAME }} ./hack/image-build.sh

# Execute a command in the boxes
batch COMMAND *ARGS:
    ./hack/box-batch.sh {{ COMMAND }} {{ ARGS }}

# List all boxes
box *ARGS:
    ./hack/box-ls.sh {{ ARGS }}

# Execute a command in a box
ssh BOX *ARGS:
    ./hack/box-ssh.sh {{ BOX }} {{ ARGS }}
