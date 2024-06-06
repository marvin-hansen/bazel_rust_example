# bin/sh
set -o errexit
set -o nounset
set -o pipefail

# Run all unit tests first
echo "=============="
echo "Run unit tests"
echo "=============="
command bazel test //... --test_tag_filters=unit
