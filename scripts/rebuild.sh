# bin/sh
set -o errexit
set -o nounset
set -o pipefail

# Syncs Cargo dependencies to Bazel index
CARGO_BAZEL_REPIN=true bazel build //...
