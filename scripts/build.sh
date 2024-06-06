# bin/sh
set -o errexit
set -o nounset
set -o pipefail


command bazel build //...