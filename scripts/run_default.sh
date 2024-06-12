# bin/sh
set -o errexit
set -o nounset
set -o pipefail

# Run default target
command bazel run //crates/grpc_server:server
