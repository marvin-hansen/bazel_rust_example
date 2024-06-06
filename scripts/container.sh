# bin/sh
set -o errexit
set -o nounset
set -o pipefail

# Builds all images
command bazel build -c opt  //:image

# Pushes all tagged images to registry
command bazel run -c opt //:push